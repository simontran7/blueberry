func fits() {
    let a: I8 = -128;
    let b: I8 = 127;
    let c: U8 = 255;
    let d: I16 = -32_768;
    let e: U16 = 65_535;
}

func out_of_range() {
    let a: I8 = -129;     // suggests I16
    let b: I8 = 200;      // suggests U8
    let c: U8 = 300;      // suggests U16
    let d: I16 = 40_000;  // suggests U16
    let e: U16 = 70_000;  // suggests U32
}

func negating_unsigned() {
    let x: U8 = 1;
    let y = -x;
}
