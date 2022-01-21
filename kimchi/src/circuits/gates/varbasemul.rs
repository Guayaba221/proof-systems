/*****************************************************************************************************************

This source file implements short Weierstrass curve variable base scalar multiplication custom Plonk constraints.

Acc := [2]T
for i = n-1 ... 0:
   Q := (r_i == 1) ? T : -T
   Acc := Acc + (Q + Acc)
return (d_0 == 0) ? Q - P : Q

One-bit round constraints:

S = (P + (b ? T : −T)) + P

VBSM gate constraints for THIS witness row
•	b1*(b1-1) = 0
•	b2*(b2-1) = 0
•	(xp - xt) * s1 = yp – (2b1-1)*yt
•	s1^2 - s2^2 = xt - xr
•	(2*xp + xt – s1^2) * (s1 + s2) = 2*yp
•	(xp – xr) * s2 = yr + yp
•	(xr - xt) * s3 = yr – (2b2-1)*yt
•	S3^2 – s4^2 = xt - xs
•	(2*xr + xt – s3^2) * (s3 + s4) = 2*yr
•	(xr – xs) * s4 = ys + yr
•	n = 32*n_n + 16*b2 + 8*b1 + 4*b3_n + 2*b2_n + b1_n

The constraints above are derived from the following EC Affine arithmetic equations:


    (xq1 - xp) * s1 = yq1 - yp
    s1^2 - s2^2 = xq1 - xr
    (2*xp + xq1 – s1^2) * (s1 + s2) = 2*yp
    (xp – xr) * s2 = yr + yp

    (xq2 - xr) * s3 = yq2 - yr
    s3^2 – s4^2 = xq2 - xs
    (2*xr + xq2 – s3^2) * (s3 + s4) = 2*yr
    (xr – xs) * s4 = ys + yr


VBSM gate constraints for NEXT witness row
•	b1*(b1-1) = 0
•	b2*(b2-1) = 0
•	b3*(b3-1) = 0
•	(xq - xp) * s1 = (2b1-1)*yt - yp
•	(2*xp – s1^2 + xq) * ((xp – xr) * s1 + yr + yp) = (xp – xr) * 2*yp
•	(yr + yp)^2 = (xp – xr)^2 * (s1^2 – xq + xr)
•	(xq - xr) * s3 = (2b2-1)*yt - yr
•	(2*xr – s3^2 + xq) * ((xr – xv) * s3 + yv + yr) = (xr – xv) * 2*yr
•	(yv + yr)^2 = (xr – xv)^2 * (s3^2 – xq + xv)
•	(xq - xv) * s5 = (2b3-1)*yt - yv
•	(2*xv – s5^2 + xq) * ((xv – xs) * s5 + ys + yv) = (xv – xs) * 2*yv
•	(ys + yv)^2 = (xv – xs)^2 * (s5^2 – xq + xs)

The constraints above are derived from the following EC Affine arithmetic equations:


    (xq1 - xp) * s1 = yq1 - yp
    s1^2 - s2^2 = xq1 - xr
    (2*xp + xq1 – s1^2) * (s1 + s2) = 2*yp
    (xp – xr) * s2 = yr + yp

    (xq2 - xr) * s3 = yq2 - yr
    s3^2 – s4^2 = xq2 - xv
    (2*xr + xq2 – s3^2) * (s3 + s4) = 2*yr
    (xr – xv) * s4 = yv + yr

    (xq3 - xv) * s5 = yq3 - yv
    s5^2 – s6^2 = xq3 - xs
    (2*xv + xq3 – s5^2) * (s5 + s6) = 2*yv
    (xv – xs) * s6 = ys + yv

=>

    (xq1 - xp) * s1 = yq1 - yp
    (2*xp – s1^2 + xq1) * ((xp – xr) * s1 + yr + yp) = (xp – xr) * 2*yp
    (yr + yp)^2 = (xp – xr)^2 * (s1^2 – xq1 + xr)

    (xq2 - xr) * s3 = yq2 - yr
    (2*xr – s3^2 + xq2) * ((xr – xv) * s3 + yv + yr) = (xr – xv) * 2*yr
    (yv + yr)^2 = (xr – xv)^2 * (s3^2 – xq2 + xv)

    (xq3 - xv) * s5 = yq3 - yv
    (2*xv – s5^2 + xq3) * ((xv – xs) * s5 + ys + yv) = (xv – xs) * 2*yv
    (ys + yv)^2 = (xv – xs)^2 * (s5^2 – xq3 + xs)


    Row	    0	1	2	3	4	5	6	7	8	9	10	11	12	13	14	Type

       i	xT	yT	xS	yS	xP	yP	n	xr	yr	s1	s2	b1	s3	s4	b2	VBSM
      i+1	s5	b3	xS	yS	xP	yP	n	xr	yr	xv	yv	s1	b1	s3	b2	ZERO

    i+100	xT	yT	xS	yS	xP	yP	n	xr	yr	s1	s2	b1	s3	s4	b2	VBSM
    i+101	s5	b3	xS	yS	xP	yP	n	xr	yr	xv	yv	s1	b1	s3	b2	ZERO


*****************************************************************************************************************/

use crate::circuits::gate::{CircuitGate, GateType};
use crate::circuits::wires::{GateWires, COLUMNS};
use ark_ff::FftField;

impl<F: FftField> CircuitGate<F> {
    pub fn create_vbmul(wires: &[GateWires; 2]) -> Vec<Self> {
        vec![
            CircuitGate {
                typ: GateType::VarBaseMul,
                wires: wires[0],
                c: vec![],
            },
            CircuitGate {
                typ: GateType::Zero,
                wires: wires[1],
                c: vec![],
            },
        ]
    }

    pub fn verify_vbmul(&self, _row: usize, _witness: &[Vec<F>; COLUMNS]) -> Result<(), String> {
        unimplemented!();
    }

    pub fn vbmul(&self) -> F {
        if self.typ == GateType::VarBaseMul {
            F::one()
        } else {
            F::zero()
        }
    }
}
