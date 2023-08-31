(declare-const x (_ BitVec 64))
(declare-const w (_ BitVec 64))
(assert (forall ((x (_ BitVec 64)) (w (_ BitVec 64)))
    (= (bvsub (bvadd x w) (bvshl (bvand x w) (_ bv1 64))) (bvxor x w))))

(check-sat)
