/// A statically-typed inductive list of mixed-type elements.
///
/// `Product` values form chains like this:
///
/// ```ignore
/// Product(1, Product(2, Product(3, ())))
/// ```
///
/// except that the elements do not all need to have the same type:
///
/// ```ignore
/// Product(1, Product(true, Product("eggs", ())))
/// ```
///
/// In `Product(h, t)`, `h` is the list element value, and `t` is the
/// tail of the list: either `()` or another `Product`.
///
/// Since `Product` implements `HList`, it has a `flatten` method that
/// turns the `Product` into an ordinary Rust tuple:
///
/// ```ignore
/// assert_eq!(Product(1, Product(true, Product("eggs", ()))).flatten(),
///            (1, true, "eggs"));
/// ```
///
/// Since `Product` also implements `Combine`, it has a `combine`
/// method that appends two lists:
///
/// ```ignore
/// let left = Product(1, Product(2, ()));
/// let right = Product("pneumonia", Product("topography", ()));
///
/// assert_eq!(left.combine(right).flatten(),
///            (1, 2, "pneumonia", "topography"));
/// ```
#[derive(Debug)]
pub struct Product<H, T: HList>(pub(crate) H, pub(crate) T);

pub type One<T> = (T,);

#[inline]
pub(crate) fn one<T>(val: T) -> One<T> {
    (val,)
}

#[derive(Debug)]
pub enum Either<T, U> {
    A(T),
    B(U),
}

/// Trait for [`Product`] chains that can be converted into a tuple.
///
/// This trait is implemented for [`Product`] chains up to 16 elements
/// long. For example:
///
/// ```ignore
/// assert_eq!(Product('a', Product(2, Product("c", ()))).flatten(),
///            ('a', 2, "c"));
/// ```
pub trait HList: Sized {
    type Tuple: Tuple<HList = Self>;

    /// Return the tuple represented by `Self`.
    fn flatten(self) -> Self::Tuple;
}

/// Trait for tuples that can be converted into a [`Product`] chain.
///
/// This trait is implemented for tuples of up to sixteen
/// elements. For example:
///
/// ```ignore
/// println!("{:?}", ('a', 2, "c").hlist());
/// ```
///
/// prints:
///
/// ```ignore
/// Product('a', Product(2, Product("c", ())))
/// ```
///
/// This trait provides a `combine` method that concatenates tuples,
/// as long as the result is no longer than sixteen elements long:
///
/// ```ignore
/// assert_eq!(('a', 2, "c").combine(('d', "e", 6)),
///            ('a', 2, "c", 'd', "e", 6));
/// ```
pub trait Tuple: Sized {
    type HList: HList<Tuple = Self>;

    fn hlist(self) -> Self::HList;

    #[inline]
    fn combine<T>(self, other: T) -> CombinedTuples<Self, T>
    where
        Self: Sized,
        T: Tuple,
        Self::HList: Combine<T::HList>,
    {
        self.hlist().combine(other.hlist()).flatten()
    }
}

/// The concatenation of two tuple types `T` and `U`.
///
/// The concatenation may have at most sixteen elements.
pub type CombinedTuples<T, U> =
    <<<T as Tuple>::HList as Combine<<U as Tuple>::HList>>::Output as HList>::Tuple;

/// Trait for `Product` lists that can be concatenated.
///
/// For example:
///
/// ```ignore
/// let left = Product(1, Product(2, ()));
/// let right = Product("pneumonia", Product("topography", ()));
///
/// assert_eq!(left.combine(right).flatten(),
///            (1, 2, "pneumonia", "topography"));
/// ```
pub trait Combine<T: HList> {
    type Output: HList;

    fn combine(self, other: T) -> Self::Output;
}

/// A function that can take its arguments from an `Args` value.
///
/// The `Func::call` method takes a function and an `Args` value,
/// and applies the function to the arguments that value represents.
///
/// This lets you apply functions of up to sixteen arguments to the
/// values carried in a [`Product`] chain or a tuple:
///
/// ```ignore
/// fn mad(a: i32, b: i32, c: i32) -> i32 { a * b + c }
///
/// assert_eq!(Func::call(&mad, (10, 20, 30)), 230);
/// ```
///
/// If `Args` is a `Product` chain or tuple of up to sixteen elements,
/// then `Func<Args>` is implemented for Rust functions and closures
/// that implement `std::ops::Fn`, whose arguments match the chain or
/// tuple's elements.
///
/// A function that accepts a single `Rejection` argument also
/// implements `Func<Rejection>`.
pub trait Func<Args> {
    type Output;

    fn call(&self, args: Args) -> Self::Output;
}

// ===== impl Combine =====

impl<T: HList> Combine<T> for () {
    type Output = T;
    #[inline]
    fn combine(self, other: T) -> Self::Output {
        other
    }
}

impl<H, T: HList, U: HList> Combine<U> for Product<H, T>
where
    T: Combine<U>,
    Product<H, <T as Combine<U>>::Output>: HList,
{
    type Output = Product<H, <T as Combine<U>>::Output>;

    #[inline]
    fn combine(self, other: U) -> Self::Output {
        Product(self.0, self.1.combine(other))
    }
}

impl HList for () {
    type Tuple = ();
    #[inline]
    fn flatten(self) -> Self::Tuple {}
}

impl Tuple for () {
    type HList = ();

    #[inline]
    fn hlist(self) -> Self::HList {}
}

impl<F, R> Func<()> for F
where
    F: Fn() -> R,
{
    type Output = R;

    #[inline]
    fn call(&self, _args: ()) -> Self::Output {
        (*self)()
    }
}

impl<F, R> Func<crate::Rejection> for F
where
    F: Fn(crate::Rejection) -> R,
{
    type Output = R;

    #[inline]
    fn call(&self, arg: crate::Rejection) -> Self::Output {
        (*self)(arg)
    }
}

/// Construct a `Product` chain from element values.
macro_rules! product {
    ($H:expr) => { Product($H, ()) };
    ($H:expr, $($T:expr),*) => { Product($H, product!($($T),*)) };
}

/// The type of a `Product` chain with the given element types.
macro_rules! Product {
    ($H:ty) => { Product<$H, ()> };
    ($H:ty, $($T:ty),*) => { Product<$H, Product!($($T),*)> };
}

/// A pattern that matches a `Product` chain whose elements match the
/// given patterns.
macro_rules! product_pat {
    ($H:pat) => { Product($H, ()) };
    ($H:pat, $($T:pat),*) => { Product($H, product_pat!($($T),*)) };
}

// Implement `HList`, `Tuple`, and `Func` for non-empty [`Product`]
// chains and tuples.
macro_rules! generics {
    ($type:ident) => {
        impl<$type> HList for Product!($type) {
            type Tuple = ($type,);

            #[inline]
            fn flatten(self) -> Self::Tuple {
                (self.0,)
            }
        }

        impl<$type> Tuple for ($type,) {
            type HList = Product!($type);
            #[inline]
            fn hlist(self) -> Self::HList {
                product!(self.0)
            }
        }

        impl<F, R, $type> Func<Product!($type)> for F
        where
            F: Fn($type) -> R,
        {
            type Output = R;

            #[inline]
            fn call(&self, args: Product!($type)) -> Self::Output {
                (*self)(args.0)
            }

        }

        impl<F, R, $type> Func<($type,)> for F
        where
            F: Fn($type) -> R,
        {
            type Output = R;

            #[inline]
            fn call(&self, args: ($type,)) -> Self::Output {
                (*self)(args.0)
            }
        }

    };

    ($type1:ident, $( $type:ident ),*) => {
        generics!($( $type ),*);

        impl<$type1, $( $type ),*> HList for Product!($type1, $($type),*) {
            type Tuple = ($type1, $( $type ),*);

            #[inline]
            fn flatten(self) -> Self::Tuple {
                #[allow(non_snake_case)]
                let product_pat!($type1, $( $type ),*) = self;
                ($type1, $( $type ),*)
            }
        }

        impl<$type1, $( $type ),*> Tuple for ($type1, $($type),*) {
            type HList = Product!($type1, $( $type ),*);

            #[inline]
            fn hlist(self) -> Self::HList {
                #[allow(non_snake_case)]
                let ($type1, $( $type ),*) = self;
                product!($type1, $( $type ),*)
            }
        }

        impl<F, R, $type1, $( $type ),*> Func<Product!($type1, $($type),*)> for F
        where
            F: Fn($type1, $( $type ),*) -> R,
        {
            type Output = R;

            #[inline]
            fn call(&self, args: Product!($type1, $($type),*)) -> Self::Output {
                #[allow(non_snake_case)]
                let product_pat!($type1, $( $type ),*) = args;
                (*self)($type1, $( $type ),*)
            }
        }

        impl<F, R, $type1, $( $type ),*> Func<($type1, $($type),*)> for F
        where
            F: Fn($type1, $( $type ),*) -> R,
        {
            type Output = R;

            #[inline]
            fn call(&self, args: ($type1, $($type),*)) -> Self::Output {
                #[allow(non_snake_case)]
                let ($type1, $( $type ),*) = args;
                (*self)($type1, $( $type ),*)
            }
        }
    };
}

generics! {
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    T7,
    T8,
    T9,
    T10,
    T11,
    T12,
    T13,
    T14,
    T15,
    T16
}
