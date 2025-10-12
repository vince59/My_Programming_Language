cls
@echo on
cls
del .\target\debug\app.wasm
del .\target\debug\app.wat
del .\target\debug\mpl.exe
del /q .\bin\*.* 
cargo build
del .\bin\mpl.exe 
copy .\target\debug\mpl.exe .\bin\mpl.exe
.\bin\mpl.exe .\examples\hello.mpl .\bin\app.wasm .\bin\app.wat

