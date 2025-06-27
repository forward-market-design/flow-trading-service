# Setup a few variables for reuse, hitting the health endpoint to get started
GET {{baseurl}}/health
[Options]
variable: bidder1="00000000-0000-0000-0000-000000000000"
variable: bidder2="00000000-0000-0000-0000-000000000001"
variable: demand_id="00000000-0000-0000-0000-000000000002"
HTTP 200


# Check GET /demand (querying for active demand curves)

# No auth header
GET {{baseurl}}/demand
HTTP 400

# No permission
GET {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder1}}
HTTP 401

# Should succeed
GET {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder1}}&can_query_bid=true
HTTP 200


# Check POST /demand (creating new demand curves)

# No auth header
POST {{baseurl}}/demand
{ "app_data": "{{demand_id}}", "curve_data": { "price": 10.0 } }
HTTP 400

# No permission
POST {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder1}}
{ "app_data": "{{demand_id}}", "curve_data": { "price": 10.0 } }
HTTP 401

# Should succeed
POST {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder1}}&can_create_bid=true
{ "app_data": "{{demand_id}}", "curve_data": { "price": 10.0 } }
HTTP 201

# Sanity check: other bidders do not see this curve
GET {{baseurl}}/demand
Authorization: Bearer bidder_id={{bidder2}}&can_query_bid=true
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$" count == 0


# Check GET /demand/{demand_id} (viewing data)

# No auth header
GET {{baseurl}}/demand/{{demand_id}}
HTTP 400

# No permission, wrong bidder
GET {{baseurl}}/demand/{{demand_id}}
Authorization: Bearer bidder_id={{bidder2}}
HTTP 401

# No permission, right bidder
GET {{baseurl}}/demand/{{demand_id}}
Authorization: Bearer bidder_id={{bidder1}}
HTTP 401

# Good permissions, wrong bidder (should fail)
GET {{baseurl}}/demand/{{demand_id}}
Authorization: Bearer bidder_id={{bidder2}}&can_read_bid=true
HTTP 401

# Good permissions, right bidder (should succeed)
GET {{baseurl}}/demand/{{demand_id}}
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200


# Check POST /demand/{demand_id} (updating demand curve)

# No auth header
PUT {{baseurl}}/demand/{{demand_id}}
HTTP 400

# No permission, wrong bidder
PUT {{baseurl}}/demand/{{demand_id}}
Authorization: Bearer bidder_id={{bidder2}}
{ "price": 5 }
HTTP 401

# No permission, right bidder
PUT {{baseurl}}/demand/{{demand_id}}
Authorization: Bearer bidder_id={{bidder1}}
{ "price": 5 }
HTTP 401

# Good permissions, wrong bidder (should fail)
PUT {{baseurl}}/demand/{{demand_id}}
Authorization: Bearer bidder_id={{bidder2}}&can_update_bid=true
{ "price": 5 }
HTTP 401

# Good permissions, right bidder (should succeed)
PUT {{baseurl}}/demand/{{demand_id}}
Authorization: Bearer bidder_id={{bidder1}}&can_update_bid=true
{ "price": 5 }
HTTP 200


# Check DELETE /demand/{demand_id} (deleting the demand curve)

# No auth header
DELETE {{baseurl}}/demand/{{demand_id}}
HTTP 400

# No permission, wrong bidder
DELETE {{baseurl}}/demand/{{demand_id}}
Authorization: Bearer bidder_id={{bidder2}}
HTTP 401

# No permission, right bidder
DELETE {{baseurl}}/demand/{{demand_id}}
Authorization: Bearer bidder_id={{bidder1}}
HTTP 401

# Good permissions, wrong bidder (should fail)
DELETE {{baseurl}}/demand/{{demand_id}}
Authorization: Bearer bidder_id={{bidder2}}&can_update_bid=true
HTTP 401

# Good permissions, right bidder (should succeed)
DELETE {{baseurl}}/demand/{{demand_id}}
Authorization: Bearer bidder_id={{bidder1}}&can_update_bid=true
HTTP 200


# Check GET /demand/{demand_id}/history

# No auth header
GET {{baseurl}}/demand/{{demand_id}}/history
HTTP 400

# No permission, wrong bidder
GET {{baseurl}}/demand/{{demand_id}}/history
Authorization: Bearer bidder_id={{bidder2}}
HTTP 401

# No permission, right bidder
GET {{baseurl}}/demand/{{demand_id}}/history
Authorization: Bearer bidder_id={{bidder1}}
HTTP 401

# Good permissions, wrong bidder
GET {{baseurl}}/demand/{{demand_id}}/history
Authorization: Bearer bidder_id={{bidder2}}&can_read_bid=true
HTTP 401

# Good permissions, right bidder
GET {{baseurl}}/demand/{{demand_id}}/history
Authorization: Bearer bidder_id={{bidder1}}&can_read_bid=true
HTTP 200
