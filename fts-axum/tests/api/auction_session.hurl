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
HTTP 200

# Create a product
POST {{baseurl}}/product
Authorization: Bearer bidder_id={{bidder1}}&can_manage_products=true
"{{product1}}"
HTTP 201

# Create a simple demand curve for bidder1
POST {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder1}}&can_create_bid=true
{
    "app_data": "{{demand1}}",
    "curve_data": { "price": 10.0 }
}
HTTP 201

# Create a simple demand curve for bidder2
POST {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder2}}&can_create_bid=true
{
    "app_data": "{{demand2}}",
    "curve_data": [{ "rate": 0, "price": 15 }, { "rate": 10, "price": 5 }]
}
HTTP 201

# Create a simple portfolio for bidder1
POST {{baseurl}}/portfolio
Authorization: Bearer bidder_id={{bidder1}}&can_create_bid=true
{
    "app_data": "{{portfolio1}}",
    "demand": { "{{demand1}}": 1 },
    "basis": { "{{product1}}": 1 }
}
HTTP 201

# Create a simple portfolio for bidder2
POST {{baseurl}}/portfolio
Authorization: Bearer bidder_id={{bidder2}}&can_create_bid=true
{
    "app_data": "{{portfolio2}}",
    "demand": { "{{demand2}}": 1 },
    "basis": { "{{product1}}": 1 }
}
HTTP 201

# Now we try to run the auction with varying degrees of success
POST {{baseurl}}/batch
HTTP 400

POST {{baseurl}}/batch
Authorization: Bearer bidder_id={{bidder1}}
HTTP 401

POST {{baseurl}}/batch
Authorization: Bearer bidder_id={{bidder1}}&can_run_batch=true
HTTP 200

# Check the product outcome

GET {{baseurl}}/product/{{product1}}/outcomes
HTTP 400

GET {{baseurl}}/product/{{product1}}/outcomes
Authorization: Bearer bidder_id={{bidder1}}
HTTP 401

GET {{baseurl}}/product/{{product1}}/outcomes
Authorization: Bearer bidder_id={{bidder1}}&can_view_products=true
HTTP 200
[Asserts]
jsonpath "$.results" count == 1
jsonpath "$.results[0].value.price" == 10
jsonpath "$.results[0].value.rate" < 5.0001
jsonpath "$.results[0].value.rate" > 4.9999

# We expect the rate to be approximately 5, but I am not clear how to do "approximately equal" comparisons in Hurl yet

GET {{baseurl}}/portfolio/{{portfolio1}}/outcomes
HTTP 400

GET {{baseurl}}/portfolio/{{portfolio1}}/outcomes
Authorization: Bearer bidder_id={{bidder1}}
HTTP 401

GET {{baseurl}}/portfolio/{{portfolio1}}/outcomes
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.results" count == 1
jsonpath "$.results[0].value.price" == 10
jsonpath "$.results[0].value.rate" > -5.0001
jsonpath "$.results[0].value.rate" < -4.9999

GET {{baseurl}}/portfolio/{{portfolio1}}/outcomes
Authorization: Bearer bidder_id={{bidder2}}&can_read_bid=true
HTTP 401

GET {{baseurl}}/portfolio/{{portfolio2}}/outcomes
Authorization: Bearer bidder_id={{bidder2}}&can_read_bid=true
HTTP 200
[Asserts]
jsonpath "$.results" count == 1
jsonpath "$.results[0].value.price" == 10
jsonpath "$.results[0].value.rate" < 5.0001
jsonpath "$.results[0].value.rate" > 4.9999