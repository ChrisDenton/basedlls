rustc basedlls.rs -C panic=abort -C link-arg=/entry:main --edition 2024 -C opt-level=3 -C link-arg=libucrt.lib -C link-arg=vcruntime.lib
./basedlls.exe
