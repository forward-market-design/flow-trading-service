NAME flow_trade_qp
ROWS
 N    gft
 E    p_A
 E    p_B
 E    g_buyer_0
 E    g_buyer_1
 E    g_buyer_2
 E    g_seller_0
 E    g_seller_1
COLUMNS
    x_buyer_X    p_A    1
    x_buyer_X    g_buyer_0    1
    x_buyer_X    g_buyer_2    1
    x_buyer_Y    p_B    1
    x_buyer_Y    g_buyer_1    1
    x_buyer_Y    g_buyer_2    1
    x_seller_X    p_A    1
    x_seller_X    g_seller_0    1
    x_seller_Y    p_B    1
    x_seller_Y    g_seller_1    1
    y_buyer_0_0    gft    -10    g_buyer_0    -1
    y_buyer_1_0    gft    -10    g_buyer_1    -1
    y_buyer_2_0    gft    -0    g_buyer_2    -1
    y_seller_0_0    gft    -6.25    g_seller_0    -1
    y_seller_1_0    gft    -6.25    g_seller_1    -1
BOUNDS
 FR BND x_buyer_X
 FR BND x_buyer_Y
 FR BND x_seller_X
 FR BND x_seller_Y
 LO BND    y_buyer_0_0    0
 UP BND    y_buyer_0_0    1
 LO BND    y_buyer_1_0    0
 UP BND    y_buyer_1_0    1
 LO BND    y_buyer_2_0    0
 UP BND    y_buyer_2_0    1
 LO BND    y_seller_0_0    -1
 UP BND    y_seller_0_0    0
 LO BND    y_seller_1_0    -1
 UP BND    y_seller_1_0    0
QUADOBJ
    y_buyer_0_0    y_buyer_0_0    5
    y_buyer_1_0    y_buyer_1_0    5
    y_buyer_2_0    y_buyer_2_0    -0
    y_seller_0_0    y_seller_0_0    -0
    y_seller_1_0    y_seller_1_0    -0
ENDATA
