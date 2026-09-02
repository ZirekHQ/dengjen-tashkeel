plugins {
    java
    id("com.vanniktech.maven.publish") version "0.30.0"
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

dependencies {
    implementation("net.java.dev.jna:jna:5.15.0")

    testImplementation(platform("org.junit:junit-bom:5.11.0"))
    testImplementation("org.junit.jupiter:junit-jupiter")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
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

mavenPublishing {
    publishToMavenCentral()
    signAllPublications()

    coordinates(group.toString(), "dengjen-tashkeel-java", version.toString())

    pom {
        name.set("dengjen-tashkeel-java")
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
                name.set("Musharraf Omer")
                email.set("ibnomer2011@hotmail.com")
            }
        }
        scm {
            url.set("https://github.com/ZirekHQ/dengjen-tashkeel")
            connection.set("scm:git:https://github.com/ZirekHQ/dengjen-tashkeel.git")
        }
    }
}
