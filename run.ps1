rustc basedlls.rs --out-dir target -C panic=abort -C link-arg=/entry:main --edition 2024 -C opt-level=3 -C lto=thin -C codegen-units=1 -C link-arg=libucrt.lib
./target/basedlls.exe
