plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
    id("org.jetbrains.kotlin.plugin.serialization")
}

// Întrebările și figurile vin din submodulul examyo-db (același ca la varianta de Windows).
val examDb = rootDir.resolve("../examyo-db")
val examAssets = layout.buildDirectory.dir("generated/examAssets")

val copyExamData by tasks.registering(Sync::class) {
    from(examDb.resolve("data/all.json"))
    from(examDb.resolve("images")) { into("images") }
    into(examAssets)
    doFirst {
        check(examDb.resolve("data/all.json").exists()) {
            "Lipsește examyo-db/data/all.json. Rulează: git submodule update --init examyo-db"
        }
    }
}

android {
    namespace = "ro.yo9va.examyo"
    compileSdk = 35

    defaultConfig {
        applicationId = "ro.yo9va.examyo"
        minSdk = 24
        targetSdk = 35
        versionCode = 2
        versionName = "0.1.1"
    }

    signingConfigs {
        // Cheia din repo e publică: ține doar ca actualizările să se instaleze peste versiunea veche.
        // Dacă în CI există variabilele ANDROID_KEYSTORE_*, se folosește cheia privată.
        create("release") {
            val ks = System.getenv("ANDROID_KEYSTORE_PATH")
            if (ks != null) {
                storeFile = file(ks)
                storePassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("ANDROID_KEY_ALIAS") ?: "examyo"
                keyPassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
            } else {
                storeFile = file("examyo-dev.jks")
                storePassword = "examyo-dev"
                keyAlias = "examyo"
                keyPassword = "examyo-dev"
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = signingConfigs.getByName("release")
        }
    }

    sourceSets["main"].assets.srcDir(examAssets)

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
    testOptions {
        unitTests.isIncludeAndroidResources = true
        unitTests.all {
            it.systemProperty("screenshots.dir", layout.buildDirectory.dir("screenshots").get().asFile.path)
            it.maxHeapSize = "2g"
        }
    }
}

tasks.named("preBuild") { dependsOn(copyExamData) }

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2025.02.00")
    implementation(composeBom)
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.activity:activity-compose:1.10.0")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.8.0")
    debugImplementation("androidx.compose.ui:ui-tooling")
    debugImplementation("androidx.compose.ui:ui-test-manifest")

    testImplementation(composeBom)
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.robolectric:robolectric:4.14.1")
    testImplementation("androidx.test:core:1.6.1")
    testImplementation("androidx.test.ext:junit:1.2.1")
    testImplementation("androidx.compose.ui:ui-test-junit4")
}
