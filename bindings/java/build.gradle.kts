@file:Suppress("UnstableApiUsage")

plugins {
    java
    `jvm-test-suite`
    `maven-publish`
    alias(libs.plugins.jreleaser)
}

group = "io.github.zirekhq"
version = "1.5.3"

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

fun expectedNativeLibraryFileName(classifier: String): String =
    when (classifier) {
        "linux-x86_64" -> "libdengjen_tashkeel_capi.so"
        "windows-x64" -> "dengjen_tashkeel_capi.dll"
        "macos-aarch64" -> "libdengjen_tashkeel_capi.dylib"
        else -> throw GradleException("unknown classifier: $classifier")
    }

val debugCdylibPath = file("${rootDir}/../../target/debug/${System.mapLibraryName("dengjen_tashkeel_capi")}")

val cargoBuildCapi = tasks.register<Exec>("cargoBuildCapi") {
    group = "build"
    description = "Builds the dengjen-tashkeel-capi debug cdylib for local test runs."
    workingDir = file("${rootDir}/../..")
    commandLine("cargo", "build", "-p", "dengjen-tashkeel-capi", "--locked")
}

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

val classpathNativeTestClassifier = hostNativeClassifier()

val stageDebugNativeArtifactForClasspathTest = tasks.register<Copy>("stageDebugNativeArtifactForClasspathTest") {
    group = "verification"
    description = "Stages the debug cdylib under the natives/<classifier>/ layout classpathNativeTest expects."
    dependsOn(cargoBuildCapi)
    from(debugCdylibPath)
    into(layout.buildDirectory.dir("classpath-native-test/$classpathNativeTestClassifier"))
    rename { expectedNativeLibraryFileName(classpathNativeTestClassifier) }
}

val classpathNativeTestJar = tasks.register<Jar>("classpathNativeTestJar") {
    group = "verification"
    description = "Packages the staged debug cdylib into a classifier jar for classpathNativeTest's runtime classpath."
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
        val test = getByName<JvmTestSuite>("test") {
            useJUnitJupiter(libs.versions.junit.jupiter.get())
        }
        val integrationTest = register<JvmTestSuite>("integrationTest") {
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
        val e2e = register<JvmTestSuite>("e2e") {
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
        register<JvmTestSuite>("classpathNativeTest") {
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
            group = "build"
            description = "Packages the $classifier native library into a classifier jar for publishing."
            archiveClassifier.set(classifier)
            val sourceDir = nativeArtifactsDir.dir(classifier)
            from(sourceDir) { into("natives/$classifier") }
            onlyIf { sourceDir.asFile.exists() }
            doFirst {
                val expectedName = expectedNativeLibraryFileName(classifier)
                val files = sourceDir.asFile.listFiles().orEmpty()
                check(files.size == 1 && files[0].isFile && files[0].name == expectedName) {
                    "expected exactly one file named '$expectedName' in $sourceDir, found ${files.toList()}"
                }
            }
        }
    }

val nativeLibraryPathOverride = project.findProperty("dengjen.tashkeel.native.library.path") as String?
val testNativeLibraryPath: String = nativeLibraryPathOverride ?: debugCdylibPath.toString()

listOf("integrationTest", "e2e").forEach { suiteName ->
    tasks.named<Test>(suiteName) {
        if (nativeLibraryPathOverride == null) {
            dependsOn(cargoBuildCapi)
        }
        systemProperty("dengjen.tashkeel.native.library.path", testNativeLibraryPath)
    }
}

tasks.withType<Test>().configureEach {
    jvmArgs("--enable-native-access=ALL-UNNAMED")
}

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
    gitRootSearch = true

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
