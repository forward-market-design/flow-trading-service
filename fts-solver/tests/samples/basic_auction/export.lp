Maximize
  gft: 10 y_buyer_0_0 + 10 y_buyer_1_0 + 0 y_buyer_2_0 + 6.25 y_seller_0_0 + 6.25 y_seller_1_0 + [ -5 y_buyer_0_0 ^ 2 + -5 y_buyer_1_0 ^ 2 + 0 y_buyer_2_0 ^ 2 + 0 y_seller_0_0 ^ 2 + 0 y_seller_1_0 ^ 2 ] / 2
Subject To
  p_A: 1 x_buyer_X + 1 x_seller_X = 0
  p_B: 1 x_buyer_Y + 1 x_seller_Y = 0
  g_buyer_0: 1 x_buyer_X - y_buyer_0_0 = 0
  g_buyer_1: 1 x_buyer_Y - y_buyer_1_0 = 0
  g_buyer_2: 1 x_buyer_X + 1 x_buyer_Y - y_buyer_2_0 = 0
  g_seller_0: 1 x_seller_X - y_seller_0_0 = 0
  g_seller_1: 1 x_seller_Y - y_seller_1_0 = 0
Bounds
  x_buyer_X free
  x_buyer_Y free
  x_seller_X free
  x_seller_Y free
  0 <= y_buyer_0_0 <= 1
  0 <= y_buyer_1_0 <= 1
  0 <= y_buyer_2_0 <= 1
  -1 <= y_seller_0_0 <= 0
  -1 <= y_seller_1_0 <= 0
End
