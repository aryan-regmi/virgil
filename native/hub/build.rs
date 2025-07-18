use std::env;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "android" {
        android();
    }
}

fn android() {
    let ndk = "~/Android/Sdk/ndk/27.0.12077973";
    let toolchain = format!("{ndk}/toolchains/llvm/prebuilt/linux-x86_64");
    let sysroot = format!("{toolchain}/sysroot");
    let bindgen_args = format!("\"--sysroot={sysroot} -I{sysroot}/usr/include\"");

    unsafe {
        env::set_var("ANDROID_NDK", &ndk);
        env::set_var("ANDROID_NDK_HOME", &ndk);
        env::set_var("BINDGEN_EXTRA_CLANG_ARGS", &bindgen_args);
    }

    println!("HERE!!!: {}", env::var("BINDGEN_EXTRA_CLANG_ARGS").unwrap());
}
