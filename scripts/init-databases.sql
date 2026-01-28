-- Initialize databases for ADI services
-- This script runs on first postgres container startup

CREATE DATABASE adi_auth;
CREATE DATABASE adi_platform;
CREATE DATABASE adi_llm_proxy;
CREATE DATABASE adi_balance;
CREATE DATABASE adi_credentials;

-- Grant privileges
GRANT ALL PRIVILEGES ON DATABASE adi_auth TO adi;
GRANT ALL PRIVILEGES ON DATABASE adi_platform TO adi;
GRANT ALL PRIVILEGES ON DATABASE adi_llm_proxy TO adi;
GRANT ALL PRIVILEGES ON DATABASE adi_balance TO adi;
GRANT ALL PRIVILEGES ON DATABASE adi_credentials TO adi;
