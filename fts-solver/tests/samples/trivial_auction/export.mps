NAME flow_trade_qp
ROWS
 N    gft
 E    p_A
 E    d_buyer
 E    d_seller
COLUMNS
    x_buyer    p_A    1
    x_buyer    d_buyer    1
    x_seller    p_A    1
    x_seller    d_seller    1
    y_buyer_0    gft    -10    d_buyer    -1
    y_seller_0    gft    -7.5    d_seller    -1
BOUNDS
 FR BND    x_buyer
 FR BND    x_seller
 LO BND    y_buyer_0    0
 UP BND    y_buyer_0    1
 LO BND    y_seller_0    -1
 UP BND    y_seller_0    0
QUADOBJ
    y_buyer_0    y_buyer_0    5
    y_seller_0    y_seller_0    -0
ENDATA
