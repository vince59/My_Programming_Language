cls
@echo on
del .\js\app.wasm
del .\js\app.wat
cargo run -- C:\rust\My_Programming_Language\examples\hello.mpl -o C:\rust\My_Programming_Language\js\app.wat
.\bin\wat2wasm .\js\app.wat -o .\js\app.wasm
node .\js\run.js
