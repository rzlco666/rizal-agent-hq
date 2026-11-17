use std::{env, path::Path, process::exit};

fn main() {
    println!("cargo:rerun-if-env-changed=HQ_WEB_DIR");
    println!("cargo:rerun-if-changed=../../web/build/index.html");

    if env::var("PROFILE").as_deref() != Ok("release") {
        return;
    }

    let build = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/build");
    let index = build.join("index.html");
    if !index.is_file() {
        eprintln!(
            "release build requires {}. Run `pnpm -C web build` first (or set HQ_WEB_DIR for dev).",
            index.display()
        );
        exit(1);
    }
    if !std::fs::read_to_string(&index)
        .is_ok_and(|html| html.contains("_app/"))
    {
        eprintln!("{} has no _app/ asset reference; run `pnpm -C web build`.", index.display());
        exit(1);
    }
}
