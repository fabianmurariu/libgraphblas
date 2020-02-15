use std::collections::HashSet;
use crate::ops::ffi::*;
use crate::*;
use crate::ops::types::*;
use crate::ops::types::desc::*;
use crate::ops::binops::*;
use crate::ops::monoid::*;
use crate::ops::vector_algebra::*;

use std::ptr;

pub fn empty_matrix_mask<B>() -> Option<&'static SparseMatrix<B>> {
    None::<&SparseMatrix<B>>
}

pub trait MatrixAlgebra<X> { // A

    fn mxm<Y, Z:TypeEncoder, B:CanBool>(
        &self,
        mask: Option<&SparseMatrix<B>>, // any type that can be made boolean
        accum: Option<&BinaryOp<Z, Z, Z>>,
        B: &SparseMatrix<Y>, // B
        s_ring: Semiring<X, Y, Z>,
        desc: &Descriptor,
    ) -> SparseMatrix<Z>; // C

    // fn mxm_mut<Y, Z, B:CanBool>(
    //     &mut self,
    //     mask: Option<&SparseMatrix<B>>, // any type that can be made boolean
    //     accum: Option<&BinaryOp<Z, Z, Z>>,
    //     B: &SparseMatrix<Y>,
    //     s_ring: Semiring<X, Y, Z>,
    //     desc: &Descriptor,
    // ) -> &mut SparseMatrix<Z>;
}

impl <X:TypeEncoder> MatrixAlgebra<X> for SparseMatrix<X> {

    fn mxm<Y, Z:TypeEncoder, B:CanBool>(
        &self, // A
        mask: Option<&SparseMatrix<B>>, // any type that can be made boolean
        accum: Option<&BinaryOp<Z, Z, Z>>,
        B: &SparseMatrix<Y>, // B
        s_ring: Semiring<X, Y, Z>,
        desc: &Descriptor,
    ) -> SparseMatrix<Z> // C 
    {
        let (m, _) = self.shape();
        let (_, n) = B.shape();
        
        let mask = mask.map(|x| x.mat).unwrap_or(ptr::null_mut::<GB_Matrix_opaque>()); 
        let acc = accum.map(|x| x.op).unwrap_or(ptr::null_mut::<GB_BinaryOp_opaque>());

        let C = SparseMatrix::<Z>::empty((m, n)); // this is actually mutated by the row below
        grb_run(||{
            unsafe{ GrB_mxm(C.mat, mask, acc, s_ring.s, self.mat, B.mat, desc.desc) }
        });
        C
    }

}

#[test]
fn multiply_2_matrices_with_mxm_for_bfs_no_transpose() {
    let mut A = SparseMatrix::<bool>::empty((7, 7));

    let edges_n:usize = 10;
    A.load(edges_n as u64, &vec![true; edges_n],
           &[0, 0, 1, 1, 2, 3, 4, 5, 6, 6],
           &[1, 3, 6, 4, 5, 4, 5, 4, 2, 3]);


    // get the neighbours for 0 and 6
    let mut B = SparseMatrix::<bool>::empty((2, 7));
    B.insert(0, 0, true);
    B.insert(1, 6, true);

    let lor_monoid = SparseMonoid::<bool>::new(BinaryOp::<bool, bool, bool>::lor(), false);
    let or_and_semi = Semiring::new(lor_monoid, BinaryOp::<bool, bool, bool>::land());

    let C = B.mxm(empty_matrix_mask::<bool>(), None, &A, or_and_semi, &Descriptor::default());
    let (r, c) = C.shape();

    // the shape must match 10x2
    assert_eq!(r, 2);
    assert_eq!(c, 7);


    let n = vec![(0,1), (0, 3), (1, 2), (1, 3)];
    let neighbours = n.iter().fold(HashSet::new(), |mut set, x| { set.insert(x); set});

    for i in 0..r {
        for j in 0..c {
            let x = C.get(i, j);
            if neighbours.contains(&(i, j)) {
                assert_eq!(x, Some(true));
            } else {
                assert_eq!(x, None);
            }
        }
    }

}

#[test]
fn graph_blas_port_bfs(){
    let s:u64 = 0; // start at 0
    let n = 7; //vertices

    let mut A = SparseMatrix::<bool>::empty((n, n));

    let edges_n:usize = 10;
    A.load(edges_n as u64, &vec![true; edges_n],
           &[0, 0, 1, 1, 2, 3, 4, 5, 6, 6],
           &[1, 3, 6, 4, 5, 2, 5, 2, 2, 3]);

    let mut v = SparseVector::<i32>::empty(n);
    let mut q = SparseVector::<bool>::empty(n);

    let mut default_desc = Descriptor::default();

    // GrB_assign (v, NULL, NULL, 0, GrB_ALL, n, NULL) ;   // make v dense
    v.assign_all(empty_mask::<bool>(), None, 0, n, &default_desc);

    //finish pending work on v
    assert_eq!(n, v.nvals());
    // GrB_Vector_setElement (q, true, s) ;   // q[s] = true, false elsewhere
    q.insert(s, true);

    // GrB_Monoid_new (&Lor, GrB_LOR, (bool) false) ;
    // GrB_Semiring_new (&Boolean, Lor, GrB_LAND) ;
    // FIXME: Semirings do not OWN monoids
    let lor_monoid = SparseMonoid::<bool>::new(BinaryOp::<bool, bool, bool>::lor(), false);
    let lor_monoid2 = SparseMonoid::<bool>::new(BinaryOp::<bool, bool, bool>::lor(), false);
    let or_and_semi = Semiring::new(lor_monoid, BinaryOp::<bool, bool, bool>::land());


    let mut desc = Descriptor::default();
    desc.set(Field::Mask, Value::SCMP).set(Field::Output, Value::Replace);

    let mut successor = true;

    let mut level:i32 = 1;
    while successor && level <= (n as i32) {
        v.assign_all(Some(&q), None, level, n, &default_desc);

        q.vxm(Some(&v), None, &A, &or_and_semi, &desc);

        q.reduce(&mut successor, None, &lor_monoid2, &default_desc);

        level = level + 1;
    }
    assert_eq!(v.get(0), Some(1));

    assert_eq!(v.get(1), Some(2));
    assert_eq!(v.get(3), Some(2));

    assert_eq!(v.get(4), Some(3));
    assert_eq!(v.get(6), Some(3));
    assert_eq!(v.get(2), Some(3));

    assert_eq!(v.get(5), Some(4));


}