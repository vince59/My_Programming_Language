cls
@echo on
del .\js\app.wasm
cargo run -- C:\rust\My_Programming_Language\examples\hello.mpl -o C:\rust\My_Programming_Language\js\app.wasm
node .\js\run.js
