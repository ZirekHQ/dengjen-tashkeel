plugins {
    java
    `maven-publish`
    alias(libs.plugins.jreleaser)
}

group = "io.github.zirekhq"
// Kept in sync by hand with [workspace.package].version in the repo root's
// Cargo.toml -- single source of truth is that file, this just mirrors it.
version = "1.5.2"

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
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
    implementation(libs.jna)

    testImplementation(platform(libs.junit.bom))
    testImplementation(libs.junit.jupiter)
    testRuntimeOnly(libs.junit.platform.launcher)
}

tasks.test {
    useJUnitPlatform()
    // Points JNA at the debug cdylib built by `cargo build -p
    // dengjen-tashkeel-capi` (repo root) so tests exercise the real FFI
    // boundary without needing a published release archive. Override with
    // -Pjna.library.path=/some/dir for a different build.
    val nativeDir = (project.findProperty("jna.library.path") as String?)
        ?: "${rootDir}/../../target/debug"
    systemProperty("jna.library.path", nativeDir)
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
