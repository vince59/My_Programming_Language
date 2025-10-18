import "lib\utils.mpl"
import "lib\unit.mpl"

main() {
 /* let integer = 3
  let float = 2.0
  let str = ""
  let bool = true */

  print("Data types :",nl, "integer=",integer,nl,"float :",float,nl,"str :",str,nl,"bool ",bool,nl)

  print("Hello from mpl !",nl) // single line comment
  call hello_from_unit()

  /* 
  multiple lines comment
  */
  print("x=", to_str(3.5),"y=",to_str(125.458),nl)
  call hello_from_utils()
  print("x=", to_str((40+4)/(2*2.3)-5.5),nl)
  print("L1", nl, "L2",nl)
  print("[","","]")
}

