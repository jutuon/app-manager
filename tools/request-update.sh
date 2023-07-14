#!/bin/bash -eux

# Example command to request update and restart
curl -H "x-api-key: password" -X POST "http://localhost:5000/manager_api/request_software_update?software_options=Backend&reboot=true"
