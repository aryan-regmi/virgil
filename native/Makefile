MAKEFLAGS += -j4

ndk_version := 27.0.12077973
abi_v7 := armeabi-v7a
abi_v8 := arm64-v8a
target_v7 := armv7-linux-androideabi
target_v8 := aarch64-linux-android
target_emulator := x86_64-linux-android
android_platform := 26

ndk_path := ~/Android/Sdk/ndk/$(ndk_version)
ndk_toolchain := $(ndk_path)/toolchains/llvm/prebuilt/linux-x86_64
sysroot = $(ndk_toolchain)/sysroot

outdir = ../android/app/src/main/jniLibs/

build_v7:
	ANDROID_NDK_HOME=$(ndk_path) \
	ANDROID_NDK=$(ndk_path) \
	CMAKE_ANDROID_ARCH_ABI=$(abi_v7) \
	cargo ndk -p $(android_platform) -t $(target_v7) build --release && \
	cp -r target/$(target_v7)/release/*.so $(outdir)/$(abi_v7)

build_v8:
	ANDROID_NDK_HOME=$(ndk_path) \
	ANDROID_NDK=$(ndk_path) \
	CMAKE_ANDROID_ARCH_ABI=$(abi_v8) \
	BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$(sysroot) -I$(sysroot)/usr/include" \
	cargo ndk -p $(android_platform) -t $(target_v8) build --release && \
	cp -r target/$(target_v8)/release/*.so $(outdir)/$(abi_v8)
	
build_emulator:
	ANDROID_NDK_HOME=$(ndk_path) \
	ANDROID_NDK=$(ndk_path) \
	CARGO_NDK_ANDROID_PLATFORM=$(android_platform) \
	CMAKE_ANDROID_ARCH_ABI=x86_64 \
	BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$(sysroot) -I$(sysroot)/usr/include" \
	cargo ndk -p $(android_platform) -t $(target_emulator) build --release && \
	cp -r target/$(target_emulator)/release/*.so $(outdir)/x86_64

build_linux:
	cargo b --release

build: build_v7 build_v8 build_emulator build_linux
	

