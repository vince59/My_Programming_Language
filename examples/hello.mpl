import "lib\utils.mpl"
import "lib\unit.mpl"

main() {
  print("Hello from mpl !") // single line comment
  call hello_from_unit()
  
#

  /* 
  multiple lines comment
  */
  print("x=", to_str(3.5),"y=",to_str(125.458))
  call hello_from_utils()
  print("x=", to_str((40+4)/(2*2.3)-5.5))
  print("L1", nl, "L2")
}

