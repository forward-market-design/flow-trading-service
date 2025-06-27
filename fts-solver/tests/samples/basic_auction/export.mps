NAME flow_trade_qp
ROWS
 N    gft
 E    p_A
 E    p_B
 E    d_buyerx
 E    d_buyery
 E    d_buyerxy
 E    d_sellerx
 E    d_sellery
COLUMNS
    x_buyerx    p_A    1
    x_buyerx    d_buyerx    1
    x_buyerx    d_buyerxy    1
    x_buyery    p_B    1
    x_buyery    d_buyery    1
    x_buyery    d_buyerxy    1
    x_sellerx    p_A    1
    x_sellerx    d_sellerx    1
    x_sellery    p_B    1
    x_sellery    d_sellery    1
    y_buyerx_0    gft    -10    d_buyerx    -1
    y_buyery_0    gft    -10    d_buyery    -1
    y_buyerxy_0    gft    -0    d_buyerxy    -1
    y_sellerx_0    gft    -6.25    d_sellerx    -1
    y_sellery_0    gft    -6.25    d_sellery    -1
BOUNDS
 FR BND    x_buyerx
 FR BND    x_buyery
 FR BND    x_sellerx
 FR BND    x_sellery
 LO BND    y_buyerx_0    0
 UP BND    y_buyerx_0    1
 LO BND    y_buyery_0    0
 UP BND    y_buyery_0    1
 LO BND    y_buyerxy_0    0
 UP BND    y_buyerxy_0    1
 LO BND    y_sellerx_0    -1
 UP BND    y_sellerx_0    0
 LO BND    y_sellery_0    -1
 UP BND    y_sellery_0    0
QUADOBJ
    y_buyerx_0    y_buyerx_0    5
    y_buyery_0    y_buyery_0    5
    y_buyerxy_0    y_buyerxy_0    -0
    y_sellerx_0    y_sellerx_0    -0
    y_sellery_0    y_sellery_0    -0
ENDATA
