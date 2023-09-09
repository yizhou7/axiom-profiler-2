(declare-const x (_ BitVec 64))
(declare-const z (_ BitVec 64))
(assert (forall ((x (_ BitVec 64)) (z (_ BitVec 64)))
    (= (bvsub (bvadd x z) (bvshl (bvand x z) (_ bv1 64))) (bvxor x z))))

(check-sat)
