plugins {
    java
    `jvm-test-suite`
    `maven-publish`
    alias(libs.plugins.jreleaser)
}

group = "io.github.zirekhq"
// Kept in sync by hand with [workspace.package].version in the repo root's
// Cargo.toml -- single source of truth is that file, this just mirrors it.
version = "1.5.2"

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(25))
    }
    withSourcesJar()
    withJavadocJar()
}

repositories {
    mavenCentral()
}

dependencyLocking {
    lockAllConfigurations()
}

dependencies {
}

val nativeClassifiers = listOf("linux-x86_64", "windows-x64", "macos-aarch64")

// Matches what java-publish.yml's "Stage native library" step produces per classifier, and what
// System.mapLibraryName("dengjen_tashkeel_capi") returns on each OS.
fun expectedNativeLibraryFileName(classifier: String): String =
    when (classifier) {
        "linux-x86_64" -> "libdengjen_tashkeel_capi.so"
        "windows-x64" -> "dengjen_tashkeel_capi.dll"
        "macos-aarch64" -> "libdengjen_tashkeel_capi.dylib"
        else -> throw GradleException("unknown classifier: $classifier")
    }

// Detects which of nativeClassifiers the machine running the build is, so the
// classpathNativeTest suite (below) knows which debug cdylib to stage and which
// natives/<classifier>/ resource path to package it under. Mirrors NativePlatform's
// runtime detection (src/main/java), duplicated here since Gradle config-time code
// can't call into the project's own compiled classes.
fun hostNativeClassifier(): String {
    val osName = System.getProperty("os.name").lowercase()
    val osArch = System.getProperty("os.arch").lowercase()
    val isArm64 = osArch == "aarch64" || osArch == "arm64"
    val isX64 = osArch == "x86_64" || osArch == "amd64" || osArch == "x64"
    return when {
        osName.contains("windows") && isX64 -> "windows-x64"
        (osName.contains("mac") || osName.contains("darwin")) && isArm64 -> "macos-aarch64"
        osName.contains("linux") && isX64 -> "linux-x86_64"
        else -> throw GradleException("unsupported host for classpathNativeTest: os=$osName arch=$osArch")
    }
}

// classpathNativeTest (registered below) proves that the natives/<classifier>/ layout
// nativeJar-<classifier> packages for release (further down this file) actually matches
// what NativeLibraryLoader resolves from the classpath at runtime -- exercised end-to-end
// against the real debug cdylib built by `cargo build -p dengjen-tashkeel-capi`, with no
// -Ddengjen.tashkeel.native.library.path override set, unlike integrationTest/e2e below.
val classpathNativeTestClassifier = hostNativeClassifier()

val stageDebugNativeArtifactForClasspathTest = tasks.register<Copy>("stageDebugNativeArtifactForClasspathTest") {
    from("${rootDir}/../../target/debug/${System.mapLibraryName("dengjen_tashkeel_capi")}")
    into(layout.buildDirectory.dir("classpath-native-test/$classpathNativeTestClassifier"))
    rename { expectedNativeLibraryFileName(classpathNativeTestClassifier) }
}

val classpathNativeTestJar = tasks.register<Jar>("classpathNativeTestJar") {
    dependsOn(stageDebugNativeArtifactForClasspathTest)
    archiveBaseName.set("dengjen-tashkeel-classpath-native-test")
    archiveClassifier.set(classpathNativeTestClassifier)
    destinationDirectory.set(layout.buildDirectory.dir("classpath-native-test"))
    from(layout.buildDirectory.dir("classpath-native-test/$classpathNativeTestClassifier")) {
        into("natives/$classpathNativeTestClassifier")
    }
}

testing {
    suites {
        val test by getting(JvmTestSuite::class) {
            useJUnitJupiter(libs.versions.junit.jupiter.get())
        }
        val integrationTest by registering(JvmTestSuite::class) {
            dependencies {
                implementation(project())
            }
            useJUnitJupiter(libs.versions.junit.jupiter.get())
            targets {
                all {
                    testTask.configure {
                        shouldRunAfter(test)
                    }
                }
            }
        }
        val e2e by registering(JvmTestSuite::class) {
            dependencies {
                implementation(project())
            }
            useJUnitJupiter(libs.versions.junit.jupiter.get())
            targets {
                all {
                    testTask.configure {
                        shouldRunAfter(integrationTest)
                    }
                }
            }
        }
        // Deliberately does NOT get the -Ddengjen.tashkeel.native.library.path override
        // that integrationTest/e2e receive below -- its only source of a native library is
        // classpathNativeTestJar on its runtime classpath, exactly mirroring a real
        // consumer's runtimeOnly classifier-jar dependency.
        val classpathNativeTest by registering(JvmTestSuite::class) {
            dependencies {
                implementation(project())
                runtimeOnly(files(classpathNativeTestJar))
            }
            useJUnitJupiter(libs.versions.junit.jupiter.get())
            targets {
                all {
                    testTask.configure {
                        shouldRunAfter(e2e)
                    }
                }
            }
        }
    }
}

tasks.named("check") {
    dependsOn(testing.suites.named("integrationTest"))
    dependsOn(testing.suites.named("e2e"))
    dependsOn(testing.suites.named("classpathNativeTest"))
}

val nativeArtifactsDir: Directory =
    layout.projectDirectory.dir((findProperty("nativeArtifactsDir") as String?) ?: "native-artifacts")

val nativeClassifierJars =
    nativeClassifiers.associateWith { classifier ->
        tasks.register<Jar>("nativeJar-$classifier") {
            archiveClassifier.set(classifier)
            val sourceDir = nativeArtifactsDir.dir(classifier)
            from(sourceDir) { into("natives/$classifier") }
            onlyIf { sourceDir.asFile.exists() }
            // Maven Central is immutable -- an empty or wrong-content classifier jar would burn
            // that version forever with a native library nobody can load, so fail loudly instead
            // of silently publishing whatever (or nothing) happens to be in the directory.
            doFirst {
                val expectedName = expectedNativeLibraryFileName(classifier)
                val files = sourceDir.asFile.listFiles().orEmpty()
                check(files.size == 1 && files[0].isFile && files[0].name == expectedName) {
                    "expected exactly one file named '$expectedName' in $sourceDir, found ${files.toList()}"
                }
            }
        }
    }

// Points FFM at the debug cdylib built by `cargo build -p
// dengjen-tashkeel-capi` (repo root) so integrationTest exercises the
// real FFI boundary without needing a published release archive.
// Override with -Pdengjen.tashkeel.native.library.path=/some/file for a
// different build. `test` needs no native library at all -- everything
// in it (NativePlatformTest, NativeLibraryLoaderTest,
// TashkeelExceptionTest) is pure Java.
val testNativeLibraryPath: String =
    (project.findProperty("dengjen.tashkeel.native.library.path") as String?)
        ?: "${rootDir}/../../target/debug/${System.mapLibraryName("dengjen_tashkeel_capi")}"

listOf("integrationTest", "e2e").forEach { suiteName ->
    tasks.named<Test>(suiteName) {
        systemProperty("dengjen.tashkeel.native.library.path", testNativeLibraryPath)
    }
}

tasks.withType<Test>().configureEach {
    jvmArgs("--enable-native-access=ALL-UNNAMED")
}

// JReleaser deploys Maven Central from a plain local Maven repo staged here by
// `publish`, rather than from Gradle's in-memory publication model directly.
val stagingDir: Provider<Directory> = layout.buildDirectory.dir("staging-deploy")

publishing {
    publications {
        create<MavenPublication>("maven") {
            from(components["java"])

            pom {
                name.set("dengjen-tashkeel")
                description.set("Java bindings for dengjen-tashkeel: Arabic-text diacritic restoration using neural networks")
                url.set("https://github.com/ZirekHQ/dengjen-tashkeel")
                licenses {
                    license {
                        name.set("MIT")
                        url.set("https://opensource.org/licenses/MIT")
                    }
                    license {
                        name.set("Apache-2.0")
                        url.set("https://www.apache.org/licenses/LICENSE-2.0")
                    }
                }
                developers {
                    developer {
                        id.set("austek")
                        name.set("Ali Ustek")
                    }
                }
                scm {
                    url.set("https://github.com/ZirekHQ/dengjen-tashkeel")
                    connection.set("scm:git:https://github.com/ZirekHQ/dengjen-tashkeel.git")
                    developerConnection.set("scm:git:git@github.com:ZirekHQ/dengjen-tashkeel.git")
                }
            }
        }
    }
    repositories {
        maven { url = uri(stagingDir.get()) }
    }
}

publishing {
    publications {
        named<MavenPublication>("maven") {
            nativeClassifierJars.values.forEach { jarTask -> artifact(jarTask) }
        }
    }
}

configure<org.jreleaser.gradle.plugin.JReleaserExtension> {
    // bindings/java is a subdirectory of this repo's git root -- without this, JReleaser's git
    // detection only looks at basedir and fails with "repository not found" instead of walking
    // up to find the repo's .git.
    gitRootSearch = true

    // GitHub releases for tagged versions are already handled by the existing cargo-dist
    // release.yml pipeline; only `jreleaserDeploy` (not `jreleaserFullRelease`) is ever invoked
    // here, so the release/changelog machinery stays unconfigured on purpose.
    signing {
        pgp {
            active = org.jreleaser.model.Active.ALWAYS
            armored = true
        }
    }
    deploy {
        maven {
            mavenCentral {
                register("sonatype") {
                    active = org.jreleaser.model.Active.ALWAYS
                    url = "https://central.sonatype.com/api/v1/publisher"
                    stagingRepository(stagingDir.get().toString())
                }
            }
        }
    }
}
