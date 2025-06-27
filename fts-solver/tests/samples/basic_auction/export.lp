Maximize
  gft: 10 y_buyerx_0 + 10 y_buyery_0 + 0 y_buyerxy_0 + 6.25 y_sellerx_0 + 6.25 y_sellery_0 + [ -5 y_buyerx_0 ^ 2 + -5 y_buyery_0 ^ 2 + 0 y_buyerxy_0 ^ 2 + 0 y_sellerx_0 ^ 2 + 0 y_sellery_0 ^ 2 ] / 2
Subject To
  p_A: 1 x_buyerx + 1 x_sellerx = 0
  p_B: 1 x_buyery + 1 x_sellery = 0
  d_buyerx: 1 x_buyerx - y_buyerx_0 = 0
  d_buyery: 1 x_buyery - y_buyery_0 = 0
  d_buyerxy: 1 x_buyerx + 1 x_buyery - y_buyerxy_0 = 0
  d_sellerx: 1 x_sellerx - y_sellerx_0 = 0
  d_sellery: 1 x_sellery - y_sellery_0 = 0
Bounds
  x_buyerx free
  x_buyery free
  x_sellerx free
  x_sellery free
  0 <= y_buyerx_0 <= 1
  0 <= y_buyery_0 <= 1
  0 <= y_buyerxy_0 <= 1
  -1 <= y_sellerx_0 <= 0
  -1 <= y_sellery_0 <= 0
End
