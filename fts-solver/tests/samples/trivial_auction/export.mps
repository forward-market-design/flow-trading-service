NAME flow_trade_qp
ROWS
 N    gft
 E    p_A
 E    g_buyer_0
 E    g_seller_0
COLUMNS
    x_buyer_X    p_A    1
    x_buyer_X    g_buyer_0    1
    x_seller_X    p_A    1
    x_seller_X    g_seller_0    1
    y_buyer_0_0    gft    -10    g_buyer_0    -1
    y_seller_0_0    gft    -7.5    g_seller_0    -1
BOUNDS
 FR BND x_buyer_X
 FR BND x_seller_X
 LO BND    y_buyer_0_0    0
 UP BND    y_buyer_0_0    1
 LO BND    y_seller_0_0    -1
 UP BND    y_seller_0_0    0
QUADOBJ
    y_buyer_0_0    y_buyer_0_0    5
    y_seller_0_0    y_seller_0_0    -0
ENDATA
