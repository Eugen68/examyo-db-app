# kotlinx.serialization: păstrează serializatoarele generate.
-keepattributes *Annotation*, InnerClasses
-dontnote kotlinx.serialization.**
-keepclassmembers class ro.yo9va.examyo.** {
    *** Companion;
}
-keepclasseswithmembers class ro.yo9va.examyo.** {
    kotlinx.serialization.KSerializer serializer(...);
}
-keep,includedescriptorclasses class ro.yo9va.examyo.**$$serializer { *; }
