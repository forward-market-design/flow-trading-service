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


# With can_query_bid, success, but no portfolios to list
GET {{baseurl}}/portfolio
Authorization: Bearer bidder_id={{bidder1}}&can_query_bid=true
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$" count == 0


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


# Now we create 2 portfolios
POST {{baseurl}}/portfolio
Authorization: Bearer bidder_id={{bidder1}}&can_create_bid=true
{
    "app_data": "{{portfolio1}}",
    "demand_group": { "{{demand1}}": 1 },
    "product_group": { "{{product1}}": 1 }
}
HTTP 201
[Asserts]
jsonpath "$.id" == "{{portfolio1}}"

POST {{baseurl}}/portfolio
Authorization: Bearer bidder_id={{bidder1}}&can_create_bid=true
{
    "app_data": "{{portfolio2}}",
    "demand_group": { "{{demand1}}": 2, "{{demand2}}": 1 },
    "product_group": { "{{product2}}": 1 }
}
HTTP 201
[Asserts]
jsonpath "$.id" == "{{portfolio2}}"

GET {{baseurl}}/portfolio/{{portfolio2}}/demand-history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.results[*]" count == 1
jsonpath "$.results[0].valid_until" == null

GET {{baseurl}}/portfolio/{{portfolio2}}/product-history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.results[*]" count == 1
jsonpath "$.results[0].valid_until" == null


# Make sure they show up
GET {{baseurl}}/portfolio
Authorization: Bearer bidder_id={{bidder1}}&can_query_bid=true
HTTP 200
[Asserts]
jsonpath "$" count == 2
jsonpath "$[*].id" contains "{{portfolio1}}"
jsonpath "$[*].id" contains "{{portfolio2}}"


# Let's make sure the portfolios are associated to the demand curves
GET {{baseurl}}/demand/{{demand1}}
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.portfolio_group[*]" count == 2
jsonpath "$.portfolio_group['{{portfolio1}}']" == 1
jsonpath "$.portfolio_group['{{portfolio2}}']" == 2

GET {{baseurl}}/demand/{{demand2}}
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.portfolio_group[*]" count == 1
jsonpath "$.portfolio_group['{{portfolio2}}']" == 1


# Let's update portfolio 2 to add product3
PATCH {{baseurl}}/portfolio/{{portfolio2}}
Authorization: Bearer bidder_id={{bidder1}}&can_update_bid=true
{ "product_group": { "{{product2}}": 1, "{{product3}}": 1 } }
HTTP 200

GET {{baseurl}}/portfolio/{{portfolio2}}/product-history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.results[*]" count == 2
jsonpath "$.results[0].valid_until" == null
jsonpath "$.results[1].valid_until" != null

GET {{baseurl}}/portfolio/{{portfolio2}}
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.product_group[*]" count == 2
jsonpath "$.product_group['{{product3}}']" == 1
jsonpath "$.demand_group[*]" count == 2

# Now we tweak the coefficient of demand1 in portfolio2
PATCH {{baseurl}}/portfolio/{{portfolio2}}
Authorization: Bearer bidder_id={{bidder1}}&can_update_bid=true
{ "demand_group": { "{{demand1}}": 1 } }
HTTP 200

GET {{baseurl}}/portfolio/{{portfolio2}}/demand-history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.results[*]" count == 2
jsonpath "$.results[0].valid_until" == null

GET {{baseurl}}/portfolio/{{portfolio2}}
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.demand_group[*]" count == 1
jsonpath "$.demand_group['{{demand1}}']" == 1

# Finally we jointly change both the demand_group and product_group
PATCH {{baseurl}}/portfolio/{{portfolio2}}
Authorization: Bearer bidder_id={{bidder1}}&can_update_bid=true
{ "demand_group": { "{{demand2}}": 1 }, "product_group": { "{{product3}}": 1 } }
HTTP 200

GET {{baseurl}}/portfolio/{{portfolio2}}/demand-history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.results[*]" count == 3
jsonpath "$.results[0].valid_until" == null

GET {{baseurl}}/portfolio/{{portfolio2}}/product-history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.results[*]" count == 3
jsonpath "$.results[0].valid_until" == null

GET {{baseurl}}/portfolio/{{portfolio2}}
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.demand_group[*]" count == 1
jsonpath "$.demand_group['{{demand2}}']" == 1
jsonpath "$.product_group[*]" count == 1
jsonpath "$.product_group['{{product3}}']" == 1

# Now we delete the portfolio
DELETE {{baseurl}}/portfolio/{{portfolio2}}
Authorization: Bearer bidder_id={{bidder1}}&can_update_bid=true
HTTP 200

GET {{baseurl}}/portfolio/{{portfolio2}}
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.demand_group[*]" count == 0
jsonpath "$.product_group[*]" count == 0

GET {{baseurl}}/portfolio/{{portfolio2}}/demand-history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.results[*]" count == 3
jsonpath "$.results[0].valid_until" != null

GET {{baseurl}}/portfolio/{{portfolio2}}/product-history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.results[*]" count == 3
jsonpath "$.results[0].valid_until" != null

# There should no portfolios associated to demand2 now

GET {{baseurl}}/demand/{{demand2}}
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.portfolio_group[*]" count == 0

# We should also only have 1 "active" portfolio
GET {{baseurl}}/portfolio
Authorization: Bearer bidder_id={{bidder1}}&can_query_bid=true
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$" count == 1
jsonpath "$[0].id" == "{{portfolio1}}"