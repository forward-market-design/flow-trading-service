# Setup a few variables for reuse, hitting the health endpoint to get started
GET {{baseurl}}/health
[Options]
variable: id1="00000000-0000-0000-0000-000000000000"
variable: id2="00000000-0000-0000-0000-000000000001"
variable: id3="00000000-0000-0000-0000-000000000002"
variable: id4="00000000-0000-0000-0000-000000000003"
variable: id5="00000000-0000-0000-0000-000000000004"
HTTP 200

# No auth header = bad request
POST {{baseurl}}/product
"{{id1}}"
HTTP 400

# Lacking permissions = not authorized
POST {{baseurl}}/product
Authorization: Bearer can_manage_products=false
"{{id1}}"
HTTP 401

# Good request = created
POST {{baseurl}}/product
Authorization: Bearer can_manage_products=true
"{{id1}}"
HTTP 201

# No auth header = bad request
GET {{baseurl}}/product/{{id1}}
HTTP 400

# No permissions = not authorized
GET {{baseurl}}/product/{{id1}}
Authorization: Bearer can_view_products=false
HTTP 401

# Good request = ok
GET {{baseurl}}/product/{{id1}}
Authorization: Bearer can_view_products=true
HTTP 200

# No permissions, no such product = 401 takes precedence
GET {{baseurl}}/product/{{id2}}
Authorization: Bearer can_view_products=false
HTTP 401

# Permissions, but no such product = 404
GET {{baseurl}}/product/{{id2}}
Authorization: Bearer can_view_products=true
HTTP 404

# No auth header = bad request
GET {{baseurl}}/product/{{id1}}/outcomes
HTTP 400

# No permissions = not authorized
GET {{baseurl}}/product/{{id1}}/outcomes
Authorization: Bearer can_view_products=false
HTTP 401

# Good request = ok
GET {{baseurl}}/product/{{id1}}/outcomes
Authorization: Bearer can_view_products=true
HTTP 200

# Good request, good permissions, non-existent id.
# Reasonably could be 200 with an empty list (a nonexistent product has no outcomes)
# or a 404. Internally, we expect outcomes to come from a different table, so
# checking for existence is an extra query. Is this performance optimization worth
# allowing a 200 here?
GET {{baseurl}}/product/{{id2}}/outcomes
Authorization: Bearer can_view_products=true
HTTP 404

# No auth header = bad request
POST {{baseurl}}/product/{{id1}}
[{ "app_data": "{{id2}}", "ratio": 1 }, { "app_data": "{{id3}}", "ratio": 1 }]
HTTP 400

# No permissions = not authorized
POST {{baseurl}}/product/{{id1}}
Authorization: Bearer can_manage_products=false
[{ "app_data": "{{id2}}", "ratio": 1 }, { "app_data": "{{id3}}", "ratio": 1 }]
HTTP 401

# Good request = created
POST {{baseurl}}/product/{{id1}}
Authorization: Bearer can_manage_products=true
[{ "app_data": "{{id2}}", "ratio": 1 }, { "app_data": "{{id3}}", "ratio": 1 }]
HTTP 201

# Good request, nonexistent product = 404
POST {{baseurl}}/product/{{id4}}
Authorization: Bearer can_manage_products=true
[{ "app_data": "{{id5}}", "ratio": 1 }]
HTTP 404
