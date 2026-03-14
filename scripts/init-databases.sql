-- Initialize databases for ADI services
-- This script runs on first postgres container startup

CREATE DATABASE adi_auth;
CREATE DATABASE adi_llm_proxy;
CREATE DATABASE adi_embed_proxy;
CREATE DATABASE adi_credentials;
CREATE DATABASE adi_payment;

-- Grant privileges
GRANT ALL PRIVILEGES ON DATABASE adi_auth TO adi;
GRANT ALL PRIVILEGES ON DATABASE adi_llm_proxy TO adi;
GRANT ALL PRIVILEGES ON DATABASE adi_embed_proxy TO adi;
GRANT ALL PRIVILEGES ON DATABASE adi_credentials TO adi;
GRANT ALL PRIVILEGES ON DATABASE adi_payment TO adi;
