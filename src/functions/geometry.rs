//! Geometry built-in function family.
//!
//! # Upstream oracle
//! - `../libqalculate/data/functions.xml.in`, Geometry category lines 4239-4588.
//! - `../libqalculate/tests/geometry.batch`

use crate::ast::Expression;
use crate::context::CalculatorContext;
use crate::functions::{
    make_unevaluated, validate_arity, BuiltinFunction, BuiltinFunctionInfo, FunctionResult,
};
use crate::number::{Number, Rational};

macro_rules! info {
    ($ident:ident, $name:literal, $args:literal, $description:literal) => {
        static $ident: BuiltinFunctionInfo = BuiltinFunctionInfo {
            name: $name,
            aliases: &[],
            min_args: $args,
            max_args: Some($args),
            description: $description,
        };
    };
}

info!(HYPOT_INFO, "hypot", 2, "Hypotenuse");
info!(TRIANGLE_INFO, "triangle", 2, "Triangle Area");
info!(
    TRIANGLE_PERIMETER_INFO,
    "triangle_perimeter", 3, "Triangle Perimeter"
);
info!(CIRCLE_INFO, "circle", 1, "Circle Area");
info!(
    CIRCUMFERENCE_INFO,
    "circumference", 1, "Circle Circumference"
);
info!(CYLINDER_INFO, "cylinder", 2, "Cylinder Volume");
info!(
    CYLINDER_SA_INFO,
    "cylinder_sa", 2, "Surface Area of Cylinder"
);
info!(CONE_INFO, "cone", 2, "Cone Volume");
info!(CONE_SA_INFO, "cone_sa", 2, "Surface Area of Cone");
info!(SPHERE_INFO, "sphere", 1, "Sphere Volume");
info!(SPHERE_SA_INFO, "sphere_sa", 1, "Surface Area of Sphere");
info!(SQUARE_INFO, "square", 1, "Square Area");
info!(
    SQUARE_PERIMETER_INFO,
    "square_perimeter", 1, "Square Perimeter"
);
info!(CUBE_INFO, "cube", 1, "Cube Volume");
info!(CUBE_SA_INFO, "cube_sa", 1, "Surface Area of Cube");
info!(RECT_INFO, "rect", 2, "Rectangle Area");
info!(
    RECT_PERIMETER_INFO,
    "rect_perimeter", 2, "Rectangle Perimeter"
);
info!(
    RECTPRISM_INFO,
    "rectprism", 3, "Volume of Rectangular Prism"
);
info!(
    RECTPRISM_SA_INFO,
    "rectprism_sa", 3, "Surface Area of Rectangular Prism"
);
info!(
    TRIANGLEPRISM_INFO,
    "triangleprism", 3, "Volume of Triangular Prism"
);
info!(PYRAMID_INFO, "pyramid", 3, "Pyramid Volume");
info!(
    TETRAHEDRON_INFO,
    "tetrahedron", 1, "Volume of Regular Tetrahedron"
);
info!(
    TETRAHEDRON_SA_INFO,
    "tetrahedron_sa", 1, "Surface Area of Regular Tetrahedron"
);
info!(
    TETRAHEDRON_HEIGHT_INFO,
    "tetrahedron_height", 1, "Height of Regular Tetrahedron"
);
info!(
    SQPYRAMID_INFO,
    "sqpyramid", 1, "Volume of Square Pyramid (Equilateral)"
);
info!(
    SQPYRAMID_SA_INFO,
    "sqpyramid_sa", 1, "Surface Area of Square Pyramid (Equilateral)"
);
info!(
    SQPYRAMID_HEIGHT_INFO,
    "sqpyramid_height", 1, "Height of Square Pyramid (Equilateral)"
);
info!(PARALLELOGRAM_INFO, "parallelogram", 2, "Parallelogram Area");
info!(
    PARALLELOGRAM_PERIMETER_INFO,
    "parallelogram_perimeter", 2, "Parallelogram Perimeter"
);
info!(TRAPEZOID_INFO, "trapezoid", 3, "Trapezoid Area");

static CATALOG: &[&BuiltinFunctionInfo] = &[
    &HYPOT_INFO,
    &TRIANGLE_INFO,
    &TRIANGLE_PERIMETER_INFO,
    &CIRCLE_INFO,
    &CIRCUMFERENCE_INFO,
    &CYLINDER_INFO,
    &CYLINDER_SA_INFO,
    &CONE_INFO,
    &CONE_SA_INFO,
    &SPHERE_INFO,
    &SPHERE_SA_INFO,
    &SQUARE_INFO,
    &SQUARE_PERIMETER_INFO,
    &CUBE_INFO,
    &CUBE_SA_INFO,
    &RECT_INFO,
    &RECT_PERIMETER_INFO,
    &RECTPRISM_INFO,
    &RECTPRISM_SA_INFO,
    &TRIANGLEPRISM_INFO,
    &PYRAMID_INFO,
    &TETRAHEDRON_INFO,
    &TETRAHEDRON_SA_INFO,
    &TETRAHEDRON_HEIGHT_INFO,
    &SQPYRAMID_INFO,
    &SQPYRAMID_SA_INFO,
    &SQPYRAMID_HEIGHT_INFO,
    &PARALLELOGRAM_INFO,
    &PARALLELOGRAM_PERIMETER_INFO,
    &TRAPEZOID_INFO,
];

/// Returns all geometry function infos.
pub fn catalog() -> Vec<&'static BuiltinFunctionInfo> {
    CATALOG.to_vec()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum GeometryOp {
    /// `hypot`: `sqrt(\x^2+\y^2)`, `functions.xml.in` lines 4241-4250.
    Hypot,
    /// `triangle`: `(\x*\y)/2`, `functions.xml.in` lines 4252-4261.
    Triangle,
    /// `triangle_perimeter`: `\x+\y+\z`, `functions.xml.in` lines 4263-4275.
    TrianglePerimeter,
    /// `circle`: `\x^2*pi`, `functions.xml.in` lines 4280-4287.
    Circle,
    /// `circumference`: `\x*2*pi`, `functions.xml.in` lines 4289-4296.
    Circumference,
    /// `cylinder`: `\x^2*pi*\y`, `functions.xml.in` lines 4301-4310.
    Cylinder,
    /// `cylinder_sa`: `2*\x^2*pi+2*pi*\x*\y`, `functions.xml.in` lines 4312-4321.
    CylinderSa,
    /// `cone`: `\x^2*pi*\y/3`, `functions.xml.in` lines 4326-4335.
    Cone,
    /// `cone_sa`: `\x^2*pi+pi*\x*abs((\y^2+\x^2)^(1/2))`, lines 4337-4346.
    ConeSa,
    /// `sphere`: `\x^3*pi*4/3`, `functions.xml.in` lines 4351-4357.
    Sphere,
    /// `sphere_sa`: `\x^2*pi*4`, `functions.xml.in` lines 4359-4365.
    SphereSa,
    /// `square`: `\x^2`, `functions.xml.in` lines 4370-4376.
    Square,
    /// `square_perimeter`: `\x*4`, `functions.xml.in` lines 4378-4384.
    SquarePerimeter,
    /// `cube`: `\x^3`, `functions.xml.in` lines 4389-4395.
    Cube,
    /// `cube_sa`: `(\x^2)*6`, `functions.xml.in` lines 4397-4403.
    CubeSa,
    /// `rect`: `\x*\y`, `functions.xml.in` lines 4408-4417.
    Rect,
    /// `rect_perimeter`: `(\x+\y)*2`, `functions.xml.in` lines 4419-4428.
    RectPerimeter,
    /// `rectprism`: `\x*\y*\z`, `functions.xml.in` lines 4433-4446.
    Rectprism,
    /// `rectprism_sa`: `(\x*\y)*2+(\x*\z)*2+(\y*\z)*2`, lines 4448-4461.
    RectprismSa,
    /// `triangleprism`: `\x*\y*\z/2`, `functions.xml.in` lines 4463-4476.
    Triangleprism,
    /// `pyramid`: `\x*\y*\z/3`, `functions.xml.in` lines 4481-4494.
    Pyramid,
    /// `tetrahedron`: `sqrt(2)/12*\x^3`, `functions.xml.in` lines 4496-4502.
    Tetrahedron,
    /// `tetrahedron_sa`: `sqrt(3)*\x^2`, `functions.xml.in` lines 4504-4510.
    TetrahedronSa,
    /// `tetrahedron_height`: `sqrt(6)/3*\x`, `functions.xml.in` lines 4512-4518.
    TetrahedronHeight,
    /// `sqpyramid`: `sqrt(2)/6*\x^3`, `functions.xml.in` lines 4520-4526.
    Sqpyramid,
    /// `sqpyramid_sa`: `(1+sqrt(3))*\x^2`, `functions.xml.in` lines 4528-4534.
    SqpyramidSa,
    /// `sqpyramid_height`: `sqrt(2)/2*\x`, `functions.xml.in` lines 4536-4542.
    SqpyramidHeight,
    /// `parallelogram`: `\x*\y`, `functions.xml.in` lines 4547-4557.
    Parallelogram,
    /// `parallelogram_perimeter`: `(\x+\y)*2`, `functions.xml.in` lines 4559-4569.
    ParallelogramPerimeter,
    /// `trapezoid`: `(\x+\y)/2*\z`, `functions.xml.in` lines 4574-4588.
    Trapezoid,
}

impl GeometryOp {
    fn info(self) -> &'static BuiltinFunctionInfo {
        match self {
            Self::Hypot => &HYPOT_INFO,
            Self::Triangle => &TRIANGLE_INFO,
            Self::TrianglePerimeter => &TRIANGLE_PERIMETER_INFO,
            Self::Circle => &CIRCLE_INFO,
            Self::Circumference => &CIRCUMFERENCE_INFO,
            Self::Cylinder => &CYLINDER_INFO,
            Self::CylinderSa => &CYLINDER_SA_INFO,
            Self::Cone => &CONE_INFO,
            Self::ConeSa => &CONE_SA_INFO,
            Self::Sphere => &SPHERE_INFO,
            Self::SphereSa => &SPHERE_SA_INFO,
            Self::Square => &SQUARE_INFO,
            Self::SquarePerimeter => &SQUARE_PERIMETER_INFO,
            Self::Cube => &CUBE_INFO,
            Self::CubeSa => &CUBE_SA_INFO,
            Self::Rect => &RECT_INFO,
            Self::RectPerimeter => &RECT_PERIMETER_INFO,
            Self::Rectprism => &RECTPRISM_INFO,
            Self::RectprismSa => &RECTPRISM_SA_INFO,
            Self::Triangleprism => &TRIANGLEPRISM_INFO,
            Self::Pyramid => &PYRAMID_INFO,
            Self::Tetrahedron => &TETRAHEDRON_INFO,
            Self::TetrahedronSa => &TETRAHEDRON_SA_INFO,
            Self::TetrahedronHeight => &TETRAHEDRON_HEIGHT_INFO,
            Self::Sqpyramid => &SQPYRAMID_INFO,
            Self::SqpyramidSa => &SQPYRAMID_SA_INFO,
            Self::SqpyramidHeight => &SQPYRAMID_HEIGHT_INFO,
            Self::Parallelogram => &PARALLELOGRAM_INFO,
            Self::ParallelogramPerimeter => &PARALLELOGRAM_PERIMETER_INFO,
            Self::Trapezoid => &TRAPEZOID_INFO,
        }
    }

    fn evaluate_numbers(self, values: &[&Number]) -> Number {
        let x = values[0];
        match self {
            Self::Hypot => squared(x).add(&squared(values[1])).sqrt(),
            Self::Triangle => x.mul(values[1]).div(&n(2)),
            Self::TrianglePerimeter => x.add(values[1]).add(values[2]),
            Self::Circle => squared(x).mul(&Number::pi()),
            Self::Circumference => x.mul(&n(2)).mul(&Number::pi()),
            Self::Cylinder => squared(x).mul(&Number::pi()).mul(values[1]),
            Self::CylinderSa => {
                let two = n(2);
                let pi = Number::pi();
                let caps = two.mul(&squared(x)).mul(&pi);
                let side = two.mul(&pi).mul(x).mul(values[1]);
                caps.add(&side)
            }
            Self::Cone => squared(x).mul(&Number::pi()).mul(values[1]).div(&n(3)),
            Self::ConeSa => {
                let pi = Number::pi();
                let base = squared(x).mul(&pi);
                let slant = squared(values[1]).add(&squared(x)).sqrt().abs();
                base.add(&pi.mul(x).mul(&slant))
            }
            Self::Sphere => {
                let four_thirds = Number::from_rational(Rational::new(4, 3));
                cubed(x).mul(&Number::pi()).mul(&four_thirds)
            }
            Self::SphereSa => squared(x).mul(&Number::pi()).mul(&n(4)),
            Self::Square => squared(x),
            Self::SquarePerimeter => x.mul(&n(4)),
            Self::Cube => cubed(x),
            Self::CubeSa => squared(x).mul(&n(6)),
            Self::Rect | Self::Parallelogram => x.mul(values[1]),
            Self::RectPerimeter | Self::ParallelogramPerimeter => x.add(values[1]).mul(&n(2)),
            Self::Rectprism => x.mul(values[1]).mul(values[2]),
            Self::RectprismSa => {
                let two = n(2);
                let xy = x.mul(values[1]).mul(&two);
                let xz = x.mul(values[2]).mul(&two);
                let yz = values[1].mul(values[2]).mul(&two);
                xy.add(&xz).add(&yz)
            }
            Self::Triangleprism => x.mul(values[1]).mul(values[2]).div(&n(2)),
            Self::Pyramid => x.mul(values[1]).mul(values[2]).div(&n(3)),
            Self::Tetrahedron => sqrt_n(2).div(&n(12)).mul(&cubed(x)),
            Self::TetrahedronSa => sqrt_n(3).mul(&squared(x)),
            Self::TetrahedronHeight => sqrt_n(6).div(&n(3)).mul(x),
            Self::Sqpyramid => sqrt_n(2).div(&n(6)).mul(&cubed(x)),
            Self::SqpyramidSa => n(1).add(&sqrt_n(3)).mul(&squared(x)),
            Self::SqpyramidHeight => sqrt_n(2).div(&n(2)).mul(x),
            Self::Trapezoid => x.add(values[1]).div(&n(2)).mul(values[2]),
        }
    }
}

struct GeometryFn(GeometryOp);

impl BuiltinFunction for GeometryFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        self.0.info()
    }

    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        let info = self.info();
        validate_arity(info.name, args, info.min_args, info.max_args)?;

        let Some(values) = numeric_args(args) else {
            return Ok(make_unevaluated(info.name, args));
        };

        Ok(Expression::Number(self.0.evaluate_numbers(&values)))
    }
}

static HYPOT_FN: GeometryFn = GeometryFn(GeometryOp::Hypot);
static TRIANGLE_FN: GeometryFn = GeometryFn(GeometryOp::Triangle);
static TRIANGLE_PERIMETER_FN: GeometryFn = GeometryFn(GeometryOp::TrianglePerimeter);
static CIRCLE_FN: GeometryFn = GeometryFn(GeometryOp::Circle);
static CIRCUMFERENCE_FN: GeometryFn = GeometryFn(GeometryOp::Circumference);
static CYLINDER_FN: GeometryFn = GeometryFn(GeometryOp::Cylinder);
static CYLINDER_SA_FN: GeometryFn = GeometryFn(GeometryOp::CylinderSa);
static CONE_FN: GeometryFn = GeometryFn(GeometryOp::Cone);
static CONE_SA_FN: GeometryFn = GeometryFn(GeometryOp::ConeSa);
static SPHERE_FN: GeometryFn = GeometryFn(GeometryOp::Sphere);
static SPHERE_SA_FN: GeometryFn = GeometryFn(GeometryOp::SphereSa);
static SQUARE_FN: GeometryFn = GeometryFn(GeometryOp::Square);
static SQUARE_PERIMETER_FN: GeometryFn = GeometryFn(GeometryOp::SquarePerimeter);
static CUBE_FN: GeometryFn = GeometryFn(GeometryOp::Cube);
static CUBE_SA_FN: GeometryFn = GeometryFn(GeometryOp::CubeSa);
static RECT_FN: GeometryFn = GeometryFn(GeometryOp::Rect);
static RECT_PERIMETER_FN: GeometryFn = GeometryFn(GeometryOp::RectPerimeter);
static RECTPRISM_FN: GeometryFn = GeometryFn(GeometryOp::Rectprism);
static RECTPRISM_SA_FN: GeometryFn = GeometryFn(GeometryOp::RectprismSa);
static TRIANGLEPRISM_FN: GeometryFn = GeometryFn(GeometryOp::Triangleprism);
static PYRAMID_FN: GeometryFn = GeometryFn(GeometryOp::Pyramid);
static TETRAHEDRON_FN: GeometryFn = GeometryFn(GeometryOp::Tetrahedron);
static TETRAHEDRON_SA_FN: GeometryFn = GeometryFn(GeometryOp::TetrahedronSa);
static TETRAHEDRON_HEIGHT_FN: GeometryFn = GeometryFn(GeometryOp::TetrahedronHeight);
static SQPYRAMID_FN: GeometryFn = GeometryFn(GeometryOp::Sqpyramid);
static SQPYRAMID_SA_FN: GeometryFn = GeometryFn(GeometryOp::SqpyramidSa);
static SQPYRAMID_HEIGHT_FN: GeometryFn = GeometryFn(GeometryOp::SqpyramidHeight);
static PARALLELOGRAM_FN: GeometryFn = GeometryFn(GeometryOp::Parallelogram);
static PARALLELOGRAM_PERIMETER_FN: GeometryFn = GeometryFn(GeometryOp::ParallelogramPerimeter);
static TRAPEZOID_FN: GeometryFn = GeometryFn(GeometryOp::Trapezoid);

/// Looks up a built-in geometry function by name.
pub fn lookup(name: &str) -> Option<&'static dyn BuiltinFunction> {
    match name {
        "hypot" => Some(&HYPOT_FN),
        "triangle" => Some(&TRIANGLE_FN),
        "triangle_perimeter" => Some(&TRIANGLE_PERIMETER_FN),
        "circle" => Some(&CIRCLE_FN),
        "circumference" => Some(&CIRCUMFERENCE_FN),
        "cylinder" => Some(&CYLINDER_FN),
        "cylinder_sa" => Some(&CYLINDER_SA_FN),
        "cone" => Some(&CONE_FN),
        "cone_sa" => Some(&CONE_SA_FN),
        "sphere" => Some(&SPHERE_FN),
        "sphere_sa" => Some(&SPHERE_SA_FN),
        "square" => Some(&SQUARE_FN),
        "square_perimeter" => Some(&SQUARE_PERIMETER_FN),
        "cube" => Some(&CUBE_FN),
        "cube_sa" => Some(&CUBE_SA_FN),
        "rect" => Some(&RECT_FN),
        "rect_perimeter" => Some(&RECT_PERIMETER_FN),
        "rectprism" => Some(&RECTPRISM_FN),
        "rectprism_sa" => Some(&RECTPRISM_SA_FN),
        "triangleprism" => Some(&TRIANGLEPRISM_FN),
        "pyramid" => Some(&PYRAMID_FN),
        "tetrahedron" => Some(&TETRAHEDRON_FN),
        "tetrahedron_sa" => Some(&TETRAHEDRON_SA_FN),
        "tetrahedron_height" => Some(&TETRAHEDRON_HEIGHT_FN),
        "sqpyramid" => Some(&SQPYRAMID_FN),
        "sqpyramid_sa" => Some(&SQPYRAMID_SA_FN),
        "sqpyramid_height" => Some(&SQPYRAMID_HEIGHT_FN),
        "parallelogram" => Some(&PARALLELOGRAM_FN),
        "parallelogram_perimeter" => Some(&PARALLELOGRAM_PERIMETER_FN),
        "trapezoid" => Some(&TRAPEZOID_FN),
        _ => None,
    }
}

fn numeric_args(args: &[Expression]) -> Option<Vec<&Number>> {
    args.iter()
        .map(|arg| match arg {
            Expression::Number(number) => Some(number),
            _ => None,
        })
        .collect()
}

fn n(value: i32) -> Number {
    Number::from_i32(value)
}

fn squared(value: &Number) -> Number {
    value.pow(&n(2))
}

fn cubed(value: &Number) -> Number {
    value.pow(&n(3))
}

fn sqrt_n(value: i32) -> Number {
    n(value).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_func(name: &str, args: &[Number]) -> Result<Number, String> {
        let func = lookup(name).ok_or_else(|| format!("function {name} not found"))?;
        let expr_args: Vec<_> = args
            .iter()
            .map(|number| Expression::Number(number.clone()))
            .collect();
        let mut ctx = CalculatorContext::default();
        match func.evaluate(&expr_args, &mut ctx) {
            Ok(Expression::Number(number)) => Ok(number),
            Ok(other) => Err(format!("expected Number, got {other:?}")),
            Err(err) => Err(err.to_string()),
        }
    }

    #[test]
    fn catalog_registers_geometry_batch_functions() {
        let names: Vec<_> = catalog().iter().map(|info| info.name).collect();
        assert_eq!(names.len(), 30);
        for expected in [
            "circle",
            "circumference",
            "cone",
            "cone_sa",
            "cube",
            "cube_sa",
            "cylinder",
            "cylinder_sa",
            "parallelogram",
            "parallelogram_perimeter",
            "rectprism",
            "rectprism_sa",
            "triangleprism",
            "tetrahedron",
            "tetrahedron_height",
            "tetrahedron_sa",
            "sqpyramid",
            "sqpyramid_height",
            "sqpyramid_sa",
            "pyramid",
            "rect",
            "rect_perimeter",
            "sphere",
            "sphere_sa",
            "square",
            "square_perimeter",
            "trapezoid",
            "triangle",
            "triangle_perimeter",
            "hypot",
        ] {
            assert!(
                lookup(expected).is_some(),
                "missing geometry function {expected}"
            );
            assert!(
                names.contains(&expected),
                "missing geometry catalog entry {expected}"
            );
        }
    }

    #[test]
    fn evaluates_geometry_batch_representatives() {
        for (name, args, expected) in [
            ("circle", vec![n(3)], "28.27433388"),
            ("circumference", vec![n(3)], "18.84955592"),
            ("cone", vec![n(3), n(4)], "37.69911184"),
            ("cube", vec![n(3)], "27"),
            ("cylinder", vec![n(3), n(4)], "113.0973355"),
            ("triangleprism", vec![n(3), n(4), n(5)], "30"),
            ("sphere", vec![n(4)], "268.0825731"),
            ("triangle", vec![n(3), n(4)], "6"),
            ("triangle_perimeter", vec![n(3), n(4), n(5)], "12"),
            ("hypot", vec![n(3), n(4)], "5"),
        ] {
            let result = eval_func(name, &args).unwrap();
            assert_eq!(result.to_qalc_string(), expected, "{name}");
        }
    }

    #[test]
    fn validates_geometry_arity() {
        assert_eq!(
            eval_func("circle", &[]).unwrap_err(),
            "circle: Expected 1 argument(s), got 0"
        );
        assert_eq!(
            eval_func("cone", &[n(3)]).unwrap_err(),
            "cone: Expected 2 argument(s), got 1"
        );
    }
}
