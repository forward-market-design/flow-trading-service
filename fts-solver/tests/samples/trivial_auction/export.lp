Maximize
  gft: 10 y_buyer_0 + 7.5 y_seller_0 + [ -5 y_buyer_0 ^ 2 + 0 y_seller_0 ^ 2 ] / 2
Subject To
  p_A: 1 x_buyer + 1 x_seller = 0
  d_buyer: 1 x_buyer - y_buyer_0 = 0
  d_seller: 1 x_seller - y_seller_0 = 0
Bounds
  x_buyer free
  x_seller free
  0 <= y_buyer_0 <= 1
  -1 <= y_seller_0 <= 0
End
