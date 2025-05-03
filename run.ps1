rustc basedlls.rs --out-dir target --edition 2024 -C panic=abort -C link-arg=/entry:main -C opt-level=3 -C lto=thin -C codegen-units=1
./target/basedlls.exe
