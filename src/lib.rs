#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::mem::{MaybeUninit};
use enum_primitive::*;
use std::marker::PhantomData;
// #[macro_use]
// extern crate lazy_static;

// lazy_static! {
//     static ref GRB:u32 = unsafe {GrB_init(GrB_Mode_GrB_NONBLOCKING)};
// }

enum_from_primitive! {
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum GrBIndex {
    Success = GrB_Info_GrB_SUCCESS,
    NoValue = GrB_Info_GrB_NO_VALUE,
}
}

pub struct SparseType {
    tpe: *mut GrB_Type,
}

pub trait TypeEncoder {
    fn blas_type() -> SparseType;
}

impl TypeEncoder for bool {
    fn blas_type() -> SparseType {
        let tpe = unsafe {&mut GrB_BOOL as *mut GrB_Type} ;
        SparseType{tpe: tpe}
    }
}

pub struct SparseMatrix<T>{
    mat: GrB_Matrix,
    _marker: PhantomData<*const T>
}

impl<T:TypeEncoder> SparseMatrix<T> {
    pub fn empty(size: (u64, u64)) -> SparseMatrix<T>{

        let mut A = MaybeUninit::<GrB_Matrix>::uninit();

        let (rows, cols) = size;
        unsafe {
            match GrB_Matrix_new(A.as_mut_ptr() , *T::blas_type().tpe, rows, cols) {
                0 => {
                    let mat = A.as_mut_ptr();
                    SparseMatrix{ mat: *mat, _marker: PhantomData , }
                },
                err => panic!("Failed to init matrix ")
            }
        }

    }

    pub fn rows(&mut self) -> u64 {
        let mut P = MaybeUninit::<u64>::uninit();

        unsafe {
            GrB_Matrix_nrows(P.as_mut_ptr(), self.mat);
        }
        unsafe{P.assume_init()}
    }


}

pub trait MatrixLike {
    type Item;

    fn insert(&mut self, row:u64, col:u64, val: Self::Item);
    fn get(&mut self, i:u64, j: u64) -> Option<Self::Item>;
}

impl<T> Drop for SparseMatrix<T> {
    fn drop(&mut self) {
        let m_pointer = &mut self.mat as *mut GrB_Matrix;
        unsafe { GrB_Matrix_free(m_pointer);}
    }
}

make_matrix_like!(bool, GrB_Matrix_extractElement_BOOL, GrB_Matrix_setElement_BOOL);
make_matrix_like!(i8, GrB_Matrix_extractElement_INT8, GrB_Matrix_setElement_INT8);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_bool_sparse_matrix() {
        unsafe {GrB_init(GrB_Mode_GrB_BLOCKING)};

        let mut m = SparseMatrix::<bool>::empty((5, 5));
        // assert!(m.rows() == 5);
        assert!(m.insert(0, 3, true) == ());
        assert!(m.insert(1, 3, true) == ());
        assert!(m.insert(2, 3, true) == ());
        assert!(m.insert(3, 3, true) == ());
        assert!(m.insert(4, 3, true) == ());
        assert!(m.get(1, 3) == Some(true));
        assert!(m.get(0, 0) == None);
        assert!(m.get(1, 3) == Some(true));
        assert!(m.get(2, 3) == Some(true));
        assert!(m.get(0, 3) == Some(true));
    }
}

#[macro_export]
macro_rules! make_matrix_like {
    ( $typ:ty, $get_elem_func:ident, $set_elem_func:ident ) => {
        impl MatrixLike for SparseMatrix<$typ> {
            type Item = $typ;

            fn insert(&mut self, row:u64, col:u64, val: Self::Item) {
                unsafe {
                    match $set_elem_func(self.mat, val, row, col) {
                        0 => (),
                        e => panic!("Failed to set element at ({}, {})={} GrB_error: {}", row, col, val, e)
                    }
                }
            }

            fn get(&mut self, i:u64, j: u64) -> Option<Self::Item> {
                let mut P = MaybeUninit::<$typ>::uninit();
                unsafe {
                    match $get_elem_func(P.as_mut_ptr(), self.mat, i, j) {
                        0 => Some(P.assume_init()),
                        1 => None,
                        e => panic!("Failed to get element at ({}, {}) GrB_error: {}", i, j, e)
                    }
                }

            }
        }
    };
}
