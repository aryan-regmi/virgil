ndk_version := 27.0.12077973

ndk_path := ~/Android/Sdk/ndk/$(ndk_version)
ndk_toolchain := $(ndk_path)/toolchains/llvm/prebuilt/linux-x86_64
sysroot = $(ndk_toolchain)/sysroot

build_apk:
	ANDROID_NDK_HOME=$(ndk_path) \
	ANDROID_NDK=$(ndk_path) \
	BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$(sysroot) -I$(sysroot)/usr/include" \
	flutter build apk
