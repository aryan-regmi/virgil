ndk_version := 27.0.12077973
abi_v7 := armeabi-v7a
abi_v8 := arm64-v8a
target_v7 := armv7-linux-androideabi
target_v8 := aarch64-linux-android
target_emulator := x86_64-linux-android

ndk_path := ~/Android/Sdk/ndk/$(ndk_version)
ndk_toolchain := $(ndk_path)/toolchains/llvm/prebuilt/linux-x86_64
sysroot = $(ndk_toolchain)/sysroot

outdir = ../android/app/src/main/jniLibs/

build_v7:
	ANDROID_NDK_HOME=$(ndk_path) \
	ANDROID_NDK=$(ndk_path) \
	CMAKE_ANDROID_ARCH_ABI=$(abi_v7) \
	cargo ndk -t $(target_v7) -o $(outdir) build --release

build_v8:
	ANDROID_NDK_HOME=$(ndk_path) \
	ANDROID_NDK=$(ndk_path) \
	CMAKE_ANDROID_ARCH_ABI=$(abi_v8) \
	BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$(sysroot) -I$(sysroot)/usr/include" \
	cargo ndk -t $(target_v8) -o $(outdir) build --release
	
build_emulator:
	ANDROID_NDK_HOME=$(ndk_path) \
	ANDROID_NDK=$(ndk_path) \
	CMAKE_ANDROID_ARCH_ABI=x86_64 \
	BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$(sysroot) -I$(sysroot)/usr/include" \
	cargo ndk -t $(target_emulator) -o $(outdir) build --release
