# Keep the JNI entry points; R8 may otherwise rename the Java class while the
# native symbol names stay fixed.
-keep class dev.oniris.audio.NativeEngine { *; }
