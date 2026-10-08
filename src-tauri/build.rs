fn main() {
    println!("cargo:rerun-if-env-changed=ZHIXIE_STORE_BUILD");
    if std::env::var("ZHIXIE_STORE_BUILD").as_deref() == Ok("1") {
        println!("cargo:rustc-env=ZHIXIE_STORE_BUILD=1");
    }
    tauri_build::build()
}
