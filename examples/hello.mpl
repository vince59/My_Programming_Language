import "lib\utils.mpl"
import "lib\unit.mpl"

main() {
  local int i
  local float f
  /*local string s = "toto"
  local bool b = true */

  // print("Data types :",nl, "i = ",i,nl,"f = ",f,nl,"s : ",s,nl,"b ",b,nl)

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

