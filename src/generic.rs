#[derive(Debug)]
pub struct Product<H, T>(pub(crate) H, pub(crate) T);

pub type One<T> = Product<T, ()>;

#[inline]
pub(crate) fn one<T>(val: T) -> One<T> {
    Product(val, ())
}

#[derive(Debug)]
pub enum Either<T, U> {
    A(T),
    B(U),
}

// Converts Product (and ()) into tuples.
pub trait HList {
    type Tuple;

    fn flatten(self) -> Self::Tuple;
}

// Combines Product together.
pub trait Combine<T> {
    type Output;

    fn combine(self, other: T) -> Self::Output;
}

pub trait Func<Args> {
    type Output;

    fn call(&self, args: Args) -> Self::Output;
}

// ===== impl Combine =====

impl<T> Combine<T> for () {
    type Output = T;
    #[inline]
    fn combine(self, other: T) -> Self::Output {
        other
    }
}

impl<H, T, U> Combine<U> for Product<H, T>
where
    T: Combine<U>,
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
    fn flatten(self) -> Self::Tuple {
        ()
    }
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

macro_rules! product {
    ($H:ty) => { Product<$H, ()> };
    ($H:ty, $($T:ty),*) => { Product<$H, product!($($T),*)> };
}

macro_rules! product_pat {
    ($H:pat) => { Product($H, ()) };
    ($H:pat, $($T:pat),*) => { Product($H, product_pat!($($T),*)) };
}

macro_rules! generics {
    ($type:ident) => {
        impl<$type> HList for product!($type) {
            type Tuple = ($type,);

            #[inline]
            fn flatten(self) -> Self::Tuple {
                (self.0,)
            }
        }

        impl<F, R, $type> Func<product!($type)> for F
        where
            F: Fn($type) -> R,
        {
            type Output = R;

            #[inline]
            fn call(&self, args: product!($type)) -> Self::Output {
                (*self)(args.0)
            }

        }

    };

    ($type1:ident, $( $type:ident ),*) => {
        generics!($( $type ),*);

        impl<$type1, $( $type ),*> HList for product!($type1, $($type),*) {
            type Tuple = ($type1, $( $type ),*);

            #[inline]
            fn flatten(self) -> Self::Tuple {
                #[allow(non_snake_case)]
                let product_pat!($type1, $( $type ),*) = self;
                ($type1, $( $type ),*)
            }
        }

        impl<F, R, $type1, $( $type ),*> Func<product!($type1, $($type),*)> for F
        where
            F: Fn($type1, $( $type ),*) -> R,
        {
            type Output = R;

            #[inline]
            fn call(&self, args: product!($type1, $($type),*)) -> Self::Output {
                #[allow(non_snake_case)]
                let product_pat!($type1, $( $type ),*) = args;
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

