@file:Suppress("UnstableApiUsage")

plugins {
    java
    `jvm-test-suite`
    `maven-publish`
    alias(libs.plugins.jreleaser)
}

group = "io.github.zirekhq"
version = "2.1.0"

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

// The javadoc tool copies ~4 MB of DejaVu fonts into every output; browsers fall back to system fonts.
tasks.named<Jar>("javadocJar") {
    exclude("resource-files/fonts/**")
}

tasks.withType<JavaCompile>().configureEach {
    options.release.set(22)
}

// The model ships once as its own artifact, so a release uploads only the small natives. Bump
// modelVersion when crates/core/data/ort/model.onnx changes; releases skip a version already on Central.
val modelVersion = "1.0.0"
val modelArtifactId = "dengjen-tashkeel-model"
val modelFile: File = rootDir.resolve("../../crates/core/data/ort/model.onnx").normalize()

val modelJar = tasks.register<Jar>("modelJar") {
    group = "build"
    description = "Packages the bundled ONNX model as the dengjen-tashkeel-model artifact."
    archiveBaseName.set(modelArtifactId)
    archiveVersion.set(modelVersion)
    from(modelFile) { into("io/github/zirekhq/dengjen/tashkeel/model") }
}

// Maven Central requires sources and javadoc jars for every artifact; these carry a note only.
fun auxiliaryJars(taskPrefix: String, artifactId: String, artifactVersion: String) =
    listOf("sources", "javadoc").map { kind ->
        tasks.register<Jar>("$taskPrefix${kind.replaceFirstChar { it.uppercase() }}Jar") {
            group = "build"
            description = "Placeholder $kind jar for the $artifactId artifact."
            archiveBaseName.set(artifactId)
            archiveVersion.set(artifactVersion)
            archiveClassifier.set(kind)
            from(resources.text.fromString("$artifactId ships binaries only; see https://github.com/ZirekHQ/dengjen-tashkeel.\n")) {
                rename { "README.txt" }
            }
        }
    }

val modelAuxiliaryJars = auxiliaryJars("model", modelArtifactId, modelVersion)

// The native libraries ship once per native change as their own artifact (all platforms in one jar),
// so a binding-only release uploads a few KB. natives.fingerprint is regenerated with
// scripts/natives-fingerprint.sh --update; bump nativesVersion whenever it changes.
val nativesVersion = "1.0.0"
val nativesArtifactId = "dengjen-tashkeel-natives"
val nativesFingerprint: String = file("natives.fingerprint").readText().trim()

val skipModelPublish = providers.gradleProperty("skipModelPublish").map(String::toBoolean).orElse(false)
val skipNativesPublish = providers.gradleProperty("skipNativesPublish").map(String::toBoolean).orElse(false)

dependencyLocking {
    lockAllConfigurations()
}

dependencies {
    // Local runs use the jar built below; the published POM names the model artifact instead (see pom.withXml).
    runtimeOnly(files(modelJar))
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
    description = "Builds the dengjen-tashkeel-capi debug cdylib, as published (ort-dylib), for test runs."
    workingDir = file("${rootDir}/../..")
    commandLine(
        "cargo", "build", "-p", "dengjen-tashkeel-capi", "--locked",
        "--no-default-features", "--features", "ort-dylib",
    )
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
                runtimeOnly(files(modelJar))
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

val publishRequested =
    gradle.startParameter.taskNames.any { it.contains("publish", ignoreCase = true) || it.contains("jreleaser", ignoreCase = true) }

val nativesJar = tasks.register<Jar>("nativesJar") {
    group = "build"
    description = "Packages every platform's native library into the dengjen-tashkeel-natives artifact."
    archiveBaseName.set(nativesArtifactId)
    archiveVersion.set(nativesVersion)
    nativeClassifiers.forEach { classifier ->
        from(nativeArtifactsDir.dir(classifier)) { into("natives/$classifier") }
    }
    onlyIf { !skipNativesPublish.get() && (nativeClassifiers.all { nativeArtifactsDir.dir(it).asFile.exists() } || publishRequested) }
    doFirst {
        nativeClassifiers.forEach { classifier ->
            val expectedName = expectedNativeLibraryFileName(classifier)
            val sourceDir = nativeArtifactsDir.dir(classifier).asFile
            val files = sourceDir.listFiles().orEmpty()
            check(files.size == 1 && files[0].isFile && files[0].name == expectedName) {
                "expected exactly one file named '$expectedName' in $sourceDir, found ${files.toList()}"
            }
        }
    }
}

val nativesAuxiliaryJars = auxiliaryJars("natives", nativesArtifactId, nativesVersion)

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

tasks.named<Test>("e2e") {
    val bundledModel = rootDir.resolve("../../crates/core/data/ort/model.onnx").normalize()
    if (System.getenv("DENGJEN_TASHKEEL_TEST_MODEL_PATH") == null && bundledModel.isFile) {
        environment("DENGJEN_TASHKEEL_TEST_MODEL_PATH", bundledModel.absolutePath)
    }
}

tasks.withType<Test>().configureEach {
    jvmArgs("--enable-native-access=ALL-UNNAMED")
}

val stagingDir: Provider<Directory> = layout.buildDirectory.dir("staging-deploy")

fun MavenPom.projectMetadata(displayName: String, summary: String) {
    name.set(displayName)
    description.set(summary)
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

publishing {
    publications {
        create<MavenPublication>("maven") {
            from(components["java"])

            pom {
                projectMetadata(
                    "dengjen-tashkeel",
                    "Java bindings for dengjen-tashkeel: Arabic-text diacritic restoration using neural networks",
                )
                withXml {
                    // runtimeOnly(files(...)) is invisible to the POM, so these dependencies are added here.
                    val dependencies = asNode().appendNode("dependencies")
                    listOf(modelArtifactId to modelVersion, nativesArtifactId to nativesVersion).forEach { (id, version) ->
                        dependencies.appendNode("dependency").apply {
                            appendNode("groupId", project.group)
                            appendNode("artifactId", id)
                            appendNode("version", version)
                            appendNode("scope", "runtime")
                        }
                    }
                }
            }
        }
        create<MavenPublication>("model") {
            artifactId = modelArtifactId
            version = modelVersion
            artifact(modelJar)
            modelAuxiliaryJars.forEach { jarTask -> artifact(jarTask) }

            pom {
                projectMetadata(
                    "dengjen-tashkeel-model",
                    "ONNX model for the dengjen-tashkeel Java bindings",
                )
            }
        }
        create<MavenPublication>("natives") {
            artifactId = nativesArtifactId
            version = nativesVersion
            artifact(nativesJar)
            nativesAuxiliaryJars.forEach { jarTask -> artifact(jarTask) }

            pom {
                projectMetadata(
                    "dengjen-tashkeel-natives",
                    "Native libraries (linux-x86_64, windows-x64, macos-aarch64) for the dengjen-tashkeel Java bindings",
                )
                // The release workflow compares this with natives.fingerprint to catch a missed version bump.
                properties.put("dengjen.natives.fingerprint", nativesFingerprint)
            }
        }
    }
    repositories {
        maven { url = uri(stagingDir.get()) }
    }
}

// Gradle module metadata would hide the POM-only model dependency from Gradle consumers.
tasks.withType<GenerateModuleMetadata>().configureEach { enabled = false }

tasks.withType<PublishToMavenRepository>().configureEach {
    onlyIf {
        when (publication.name) {
            "model" -> !skipModelPublish.get()
            "natives" -> !skipNativesPublish.get()
            else -> true
        }
    }
}

tasks.withType<PublishToMavenLocal>().configureEach {
    onlyIf {
        when (publication.name) {
            "model" -> !skipModelPublish.get()
            "natives" -> !skipNativesPublish.get()
            else -> true
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
