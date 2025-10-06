cls
@echo on
del .\js\app.wasm
del .\js\app.wat
cargo run -- C:\rust\My_Programming_Language\examples\hello.mpl C:\rust\My_Programming_Language\js\app.wasm C:\rust\My_Programming_Language\js\app.wat
node .\js\run.js
