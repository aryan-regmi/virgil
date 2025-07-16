
ndk := ~/Android/Sdk/ndk/27.0.12077973
cmake := ~/Android/Sdk/cmake/4.0.2/bin/cmake
llvm_toolchain := $(ndk)/toolchains/llvm/prebuilt/linux-x86_64
ndk_toolchain := $(ndk)/prebuilt/linux-x86_64


# CMAKE_TOOLCHAIN_FILE=$(ndk)/build/cmake/android.toolchain.cmake

run:
	CMAKE_BUILD_TYPE=Release \
  CMAKE_TOOLCHAIN_FILE=~/Android/Sdk/ndk/27.0.12077973/build/cmake/android.toolchain.cmake \
  CMAKE_INSTALL_PREFIX=. \
  CMAKE_CXX_FLAGS="-O3 -std=c++17 -march=armv7-a" \
  CMAKE_C_FLAGS="-O3 -march=armv7-a" \
	BINDGEN_EXTRA_CLANG_ARGS=-DCMAKE_C_FLAGS="-O3 -march=armv7-a" \
  ANDROID_ABI=armeabi-v7a \
  ANDROID_NATIVE_API_LEVEL=android-34 \
  ANDROID_PLATFORM=android-34 \
  CMAKE_FIND_ROOT_PATH_MODE_PACKAGE=ON \
  ANDROID_STL=c++_static \
	flutter build apk

apk:
	ANDROID_NDK=$(ndk) \
	ANDROID_NDK_HOME=$(ndk) \
	ANDROID_NDK_ROOT=$(ndk) \
	ANDROID_PLATFORM=24 \
	ANDROID_ABI=armeabi-v7a \
	ANDROID_STANDALONE_TOOLCHAIN=$(llvm_toolchain) \
	CMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY \
	CMAKE_C_COMPILER_WORKS=1 \
	CMAKE_SYSTEM_PROCESSOR=armv7a \
	CMAKE_x86_64_linux_android=$(cmake) \
	CMAKE_MAKE_PROGRAM=$(ndk_toolchain)/bin/make \
	CMAKE_C_COMPILER=$(llvm_toolchain)/bin/armv7a-linux-androideabi24-clang \
	CMAKE_CXX_COMPILER=$(llvm_toolchain)/bin/armv7a-linux-androideabi24-clang++ \
	CMAKE_ANDROID_ARCH_ABI=arm64-v8a \
	BINDGEN_EXTRA_CLANG_ARGS=--sysroot=$(llvm_toolchain)/sysroot,-DWHISPER_BUILD_EXAMPLES=0,-DCMAKE_ANDROID_ARCH_ABI=arm64-v8a,-DCMAKE_ASM_COMPILER=$(llvm_toolchain)/bin/armv7a-linux-androideabi24-clang,-DCMAKE_C_FLAGS="-march=armv7-a" \
 \
	WHISPER_BUILD_EXAMPLES=0 \
	flutter build apk
	
