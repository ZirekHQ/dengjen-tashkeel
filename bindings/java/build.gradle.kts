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
    }
}

tasks.named("check") {
    dependsOn(testing.suites.named("integrationTest"))
    dependsOn(testing.suites.named("e2e"))
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
