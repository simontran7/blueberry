# Methods and Traits

## Methods

> [!CAUTION]
> Unimplemented

```
record Point {
    x: I32,
    y: I32,
}

impl Point {
    func origin() -> Point {
        Point { x: 0, y: 0 }
    }

    func length_squared(self) -> I32 {
        self.x * self.x + self.y * self.y
    }
}

func main() {
    let p = Point::origin();
    let a = p.length_squared();
    let b = Point::length_squared(p);
}
```

An `impl` block holds the functions of a type, and works for any named type: named records, named tuples, and variants. You call them through the type's name, like `Point::origin()`.

A function whose first parameter is `self` is a method: you can also call it on a value with a `.`, and `self` is that value. `p.length_squared()` is just a shorter way to write `Point::length_squared(p)`.

## Traits

> [!CAUTION]
> Unimplemented

```
trait Area {
    func area(self) -> I32;
}

impl Area for Point {
    func area(self) -> I32 {
        0
    }
}

impl Area for Shape {
    func area(self) -> I32 {
        match self {
            Shape::Point => 0,
            Shape::Circle(r) => 3 * r * r,
            Shape::Rectangle { width, height } => width * height,
        }
    }
}
```

A trait names a set of functions that several types can have. `trait Area` declares `area` without a body, ending with a `;`, and each `impl Area for …` gives a type its own `area`. Then `p.area()` and `s.area()` both work.

An `impl` of a trait must define every function of the trait. A trait can also give a function a body, as a default: a type that doesn't define that function gets the default instead.

You can implement a trait for a type you didn't declare, like `impl Area for I32`, but only in the module that declares the trait or the type. That way, there's never more than one `impl` of a trait for a type, wherever it's used.

## Associated Constants

> [!CAUTION]
> Unimplemented

```
impl Point {
    const ORIGIN: Point = Point { x: 0, y: 0 };
}

record Square {
    side: I32,
}

trait Polygon {
    const SIDES: I32;
}

impl Polygon for Square {
    const SIDES: I32 = 4;
}

func main() {
    let o = Point::ORIGIN;
    let n = Square::SIDES;
}
```

An `impl` block can hold constants too: they belong to the type, and you reach them through its name, like `Point::ORIGIN`.

A trait can declare a constant without a value, which every `impl` of the trait must give, like `SIDES`. A trait constant with a value is a default, like a trait function with a body.
