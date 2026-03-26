# MySQL Sales Reporter Hand

This hand automates the process of querying a MySQL database for sales data and emailing reports.

## Setup Requirements

1. **MySQL CLI**: Install the `mysql` client.
2. **Himalaya**: Configure `himalaya` for your email account.
3. **Env Vars**:
   - `MYSQL_USER`: Database username.
   - `MYSQL_PASSWORD`: Database password.
   - `MYSQL_DB`: Database name.

## Usage

Describe your reporting needs to the Setup Wizard. For example:
"Email me the top 10 sales every day at 5 PM to `boss@example.com`"

The wizard will wake this hand up on the defined schedule.
