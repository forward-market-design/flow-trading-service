Maximize
  gft: 10 y_buyer_0_0 + 7.5 y_seller_0_0 + [ -5 y_buyer_0_0 ^ 2 + 0 y_seller_0_0 ^ 2 ] / 2
Subject To
  p_A: 1 x_buyer_X + 1 x_seller_X = 0
  g_buyer_0: 1 x_buyer_X - y_buyer_0_0 = 0
  g_seller_0: 1 x_seller_X - y_seller_0_0 = 0
Bounds
  x_buyer_X free
  x_seller_X free
  0 <= y_buyer_0_0 <= 1
  -1 <= y_seller_0_0 <= 0
End
