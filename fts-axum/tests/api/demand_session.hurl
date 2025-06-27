# Setup a few variables for reuse, hitting the health endpoint to get started
GET {{baseurl}}/health
[Options]
variable: bidder1="00000000-0000-0000-0000-000000000000"
variable: bidder2="00000000-0000-0000-0000-000000000001"
variable: demand_id="00000000-0000-0000-0000-000000000002"
HTTP 200


# Query the active demands (of which there are none)
GET {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder1}}&can_query_bid=true
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$" count == 0


# Create a simple demand curve, but invalid
POST {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder1}}&can_create_bid=true
{
    "app_data": "{{demand_id}}",
    "curve_data": {
        "min_rate": 5,
        "max_rate": 10,
        "price": 10.0
    }
}
HTTP 422


# Create a simple demand curve successfully
POST {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder1}}&can_create_bid=true
{
    "app_data": "{{demand_id}}",
    "curve_data": {
        "min_rate": 0,
        "max_rate": 10,
        "price": 10.0
    }
}
HTTP 201
[Asserts]
jsonpath "$.demand_id" == "{{demand_id}}"


# Verify the bid is considered live
GET {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder1}}&can_query_bid=true
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$" count == 1
jsonpath "$[0]" == "{{demand_id}}"


# Ensure we have a history entry
GET {{baseurl}}/demand/{{demand_id}}/history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$.results" count == 1


# Check the multi-bidder access
GET {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder1}}&bidder_id={{bidder2}}&can_query_bid=true
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$" count == 1


# Update the bid
PUT {{baseurl}}/demand/{{demand_id}}
Authorization: Bearer bidder_id={{bidder1}}&can_update_bid=true
{ "min_rate": -1, "max_rate": 1, "price": 5 }
HTTP 200


# Delete the bid
DELETE {{baseurl}}/demand/{{demand_id}}
Authorization: Bearer bidder_id={{bidder1}}&can_update_bid=true
HTTP 200


# We now have no live demand curves
GET {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder1}}&can_query_bid=true
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$" count == 0


# We should have (1) the initial creation, (2) the updated record, and (3) the NULL
# entries in the demand history
GET {{baseurl}}/demand/{{demand_id}}/history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$.results" count == 2
jsonpath "$.results[0].valid_until" != null
