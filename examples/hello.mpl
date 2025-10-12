import "lib\utils.mpl"
import "lib\unit.mpl"

main() {
  print("Hello from mpl !")
  call hello_from_unit()
  call hello_from_utils()
  print("x=", to_str((40+4)/(2*2)-5))
}

