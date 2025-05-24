plugins {
    kotlin("jvm") version "1.9.21"
}

group = "cc.getportal"
version = "0.1.0"

repositories {
    mavenCentral()
}

dependencies {
    testImplementation("org.jetbrains.kotlin:kotlin-test")

    api(group = "org.slf4j", name = "slf4j-api", version = "2.0.17")
    implementation(group = "org.slf4j", name = "slf4j-simple", version = "2.0.17")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")

    implementation("com.fasterxml.jackson.module:jackson-module-kotlin:2.17.0")
    implementation("com.fasterxml.jackson.core:jackson-databind:2.17.0")
    implementation("com.fasterxml.jackson.core:jackson-annotations:2.17.0")
}

tasks.test {
    useJUnitPlatform()
}
kotlin {
    jvmToolchain(11)
}

tasks.jar {
    archiveBaseName.set("portal-sdk")
}
