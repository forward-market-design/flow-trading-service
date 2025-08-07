# Setup a few variables for reuse, hitting the health endpoint to get started
GET {{baseurl}}/health
[Options]
variable: bidder1="00000000-0000-0000-0000-000000000000"
variable: bidder2="00000000-0000-0000-0000-000000000001"
variable: demand1="00000000-0000-0000-0000-100000000000"
variable: demand2="00000000-0000-0000-0000-100000000001"
variable: portfolio1="00000000-0000-0000-0000-200000000000"
variable: portfolio2="00000000-0000-0000-0000-200000000001"
variable: product1="00000000-0000-0000-0000-300000000000"
variable: product2="00000000-0000-0000-0000-300000000001"
variable: product3="00000000-0000-0000-0000-300000000002"
HTTP 200

# Create two simple demand curves for use with portfolios
POST {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder1}}&can_create_bid=true
{
    "app_data": "{{demand1}}",
    "curve_data": {
        "min_rate": -10,
        "max_rate": 10,
        "price": 10.0
    }
}
HTTP 201

POST {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder1}}&can_create_bid=true
{
    "app_data": "{{demand2}}",
    "curve_data": {
        "min_rate": -5,
        "max_rate": 5,
        "price": 5.0
    }
}
HTTP 201

# We need to also create a few products
POST {{baseurl}}/product
Authorization: Bearer bidder_id={{bidder1}}&can_manage_products=true
"{{product1}}"
HTTP 201

POST {{baseurl}}/product
Authorization: Bearer bidder_id={{bidder1}}&can_manage_products=true
"{{product2}}"
HTTP 201

POST {{baseurl}}/product
Authorization: Bearer bidder_id={{bidder1}}&can_manage_products=true
"{{product3}}"
HTTP 201


# Now we test the portfolio endpoints

# The "active portfolios" endpoint

GET {{baseurl}}/portfolio
HTTP 400

GET {{baseurl}}/portfolio
Authorization: Bearer bidder_id={{bidder1}}
HTTP 401

GET {{baseurl}}/portfolio
Authorization: Bearer bidder_id={{bidder1}}&can_query_bid=true
HTTP 200

# The "create a portfolio" endpoint

POST {{baseurl}}/portfolio
{
    "app_data": "{{portfolio1}}",
    "demand": { "{{demand1}}": 1 },
    "basis": { "{{product1}}": 1 }
}
HTTP 400

POST {{baseurl}}/portfolio
Authorization: Bearer bidder_id={{bidder1}}
{
    "app_data": "{{portfolio1}}",
    "demand": { "{{demand1}}": 1 },
    "basis": { "{{product1}}": 1 }
}
HTTP 401

POST {{baseurl}}/portfolio
Authorization: Bearer bidder_id={{bidder1}}&can_create_bid=true
{
    "app_data": "{{portfolio1}}",
    "demand": { "{{demand1}}": 1 },
    "basis": { "{{product1}}": 1 }
}
HTTP 201

# The "get a specific portfolio" endpoint

GET {{baseurl}}/portfolio/{{portfolio1}}
HTTP 400

GET {{baseurl}}/portfolio/{{portfolio1}}
Authorization: Bearer bidder_id={{bidder1}}
HTTP 401

GET {{baseurl}}/portfolio/{{portfolio1}}
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200

# TODO: maybe this should be 401, but the way the querying works this is the more natural response
GET {{baseurl}}/portfolio/{{portfolio2}}
Authorization: Bearer bidder_id={{bidder1}}
HTTP 404

GET {{baseurl}}/portfolio/{{portfolio2}}
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 404

# The "update the portfolio" endpoint

PATCH {{baseurl}}/portfolio/{{portfolio1}}
{
    "demand": { "{{demand1}}": 1, "{{demand2}}": 1 },
    "basis": { "{{product1}}": 1, "{{product2}}": 1 }
}
HTTP 400

PATCH {{baseurl}}/portfolio/{{portfolio1}}
Authorization: Bearer bidder_id={{bidder1}}
{
    "demand": { "{{demand1}}": 1, "{{demand2}}": 1 },
    "basis": { "{{product1}}": 1, "{{product2}}": 1 }
}
HTTP 401

PATCH {{baseurl}}/portfolio/{{portfolio1}}
Authorization: Bearer bidder_id={{bidder1}}&can_update_bid=true
{
    "demand": { "{{demand1}}": 1, "{{demand2}}": 1 },
    "basis": { "{{product1}}": 1, "{{product2}}": 1 }
}
HTTP 200

# TODO: this probably should be a 401?
PATCH {{baseurl}}/portfolio/{{portfolio2}}
Authorization: Bearer bidder_id={{bidder1}}
{
    "demand": { "{{demand1}}": 1, "{{demand2}}": 1 },
    "basis": { "{{product1}}": 1, "{{product2}}": 1 }
}
HTTP 404

PATCH {{baseurl}}/portfolio/{{portfolio2}}
Authorization: Bearer bidder_id={{bidder1}}&can_update_bid=true
{
    "demand": { "{{demand1}}": 1, "{{demand2}}": 1 },
    "basis": { "{{product1}}": 1, "{{product2}}": 1 }
}
HTTP 404

# The "delete the portfolio" endpoint

DELETE {{baseurl}}/portfolio/{{portfolio1}}
HTTP 400

DELETE {{baseurl}}/portfolio/{{portfolio1}}
Authorization: Bearer bidder_id={{bidder1}}
HTTP 401

DELETE {{baseurl}}/portfolio/{{portfolio1}}
Authorization: Bearer bidder_id={{bidder1}}&can_update_bid=true
HTTP 200

# TODO: this probably should be a 401?
DELETE {{baseurl}}/portfolio/{{portfolio2}}
Authorization: Bearer bidder_id={{bidder1}}
HTTP 404

DELETE {{baseurl}}/portfolio/{{portfolio2}}
Authorization: Bearer bidder_id={{bidder1}}&can_update_bid=true
HTTP 404

# The history of the demand groups associated to the portfolio

GET {{baseurl}}/portfolio/{{portfolio1}}/demand-history
HTTP 400

GET {{baseurl}}/portfolio/{{portfolio1}}/demand-history
Authorization: Bearer bidder_id={{bidder1}}
HTTP 401

GET {{baseurl}}/portfolio/{{portfolio1}}/demand-history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200

# TODO: maybe this should be 401, but the way the querying works this is the more natural response
GET {{baseurl}}/portfolio/{{portfolio2}}/demand-history
Authorization: Bearer bidder_id={{bidder1}}
HTTP 404

GET {{baseurl}}/portfolio/{{portfolio2}}/demand-history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 404

# The history of the product groups associated to the portfolio

GET {{baseurl}}/portfolio/{{portfolio1}}/product-history
HTTP 400

GET {{baseurl}}/portfolio/{{portfolio1}}/product-history
Authorization: Bearer bidder_id={{bidder1}}
HTTP 401

GET {{baseurl}}/portfolio/{{portfolio1}}/product-history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200

# TODO: maybe this should be 401, but the way the querying works this is the more natural response
GET {{baseurl}}/portfolio/{{portfolio2}}/product-history
Authorization: Bearer bidder_id={{bidder1}}
HTTP 404

GET {{baseurl}}/portfolio/{{portfolio2}}/product-history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 404

# The final remaining endpoint:
# GET /portfolio/{id}/outcomes
# is deferred to the auction tests.