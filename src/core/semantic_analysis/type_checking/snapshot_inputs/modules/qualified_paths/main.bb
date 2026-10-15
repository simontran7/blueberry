import shapes::circle;
import shapes::nope;
import ghost;

func unit_area() -> I32 {
    circle::area(1)
}

const DOUBLE_PI: I32 = circle::PI * 2;

func errors(x: I32) {
    let a = circle::nope;
    circle::missing(1);
    let b = nope::area;
    let c = circle;
    let d = unit_area::x;
    let e = x::y;
    let f = area(1);
    circle::PI = 4;
    let g = ghost::thing;
    let h = undeclared::area;
}
