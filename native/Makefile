ndk_version := 27.0.12077973
abi := armeabi-v7a
# abi := arm64-v8a

ndk_path := ~/Android/Sdk/ndk/$(ndk_version)
ndk_toolchain := $(ndk_path)/toolchains/llvm/prebuilt/linux-x86_64

build:
	ANDROID_NDK_HOME=$(ndk_path) \
	ANDROID_NDK=$(ndk_path) \
	CMAKE_ANDROID_ARCH_ABI=$(abi) \
	BINDGEN_EXTRA_ARGS=--sysroot=$(ndk_toolchain)/sysroot \
	cargo ndk -t $(abi) build --release
