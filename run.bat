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
.\bin\mpl.exe -c .\examples\hello.mpl -o .\bin\app.wasm -a .\bin\app.wat
.\bin\mpl.exe -r .\examples\hello.mpl
