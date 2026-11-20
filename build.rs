// Copyright (C) 2026 Nexulon Corp. All rights reserved.
// See LICENSE for the terms.

fn main() {
    println!("cargo:rerun-if-changed=assets/nexulon.rc");
    println!("cargo:rerun-if-changed=assets/nexulon.ico");
    println!("cargo:rerun-if-changed=assets/nexulon.png");
    println!("cargo:rerun-if-changed=assets/platforms");
    println!("cargo:rerun-if-changed=data/Platforms.default.json");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let _ = embed_resource::compile("assets/nexulon.rc", embed_resource::NONE);
    }
}
