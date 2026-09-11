# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.
#
# For more details, see
#   http://developer.android.com/guide/developing/tools/proguard.html

# Keep custom Tauri Android plugins: they are instantiated by name and their
# @Command methods are looked up via reflection, so R8 must not strip/rename them.
-keep class com.mankong.sendsent.ContentPlugin { *; }
-keep class com.mankong.sendsent.NsdPlugin { *; }
-keepclassmembers class * {
  @app.tauri.annotation.Command <methods>;
}


# If your project uses WebView with JS, uncomment the following
# and specify the fully qualified class name to the JavaScript interface
# class:
#-keepclassmembers class fqcn.of.javascript.interface.for.webview {
#   public *;
#}

# Uncomment this to preserve the line number information for
# debugging stack traces.
#-keepattributes SourceFile,LineNumberTable

# If you keep the line number information, uncomment this to
# hide the original source file name.
#-renamesourcefileattribute SourceFile