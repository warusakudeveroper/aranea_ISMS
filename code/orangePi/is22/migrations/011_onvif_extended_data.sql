-- Migration 011: Add extended ONVIF data fields
-- Store full ONVIF discovery data for display in UI

-- GetScopes data (name, hardware, profile, location, type)
ALTER TABLE cameras ADD COLUMN onvif_scopes JSON DEFAULT NULL;

-- GetNetworkInterfaces data (interfaces with MAC, IP info)
ALTER TABLE cameras ADD COLUMN onvif_network_interfaces JSON DEFAULT NULL;

-- GetCapabilities data (Analytics, Device, Events, Imaging, Media, PTZ details)
ALTER TABLE cameras ADD COLUMN onvif_capabilities JSON DEFAULT NULL;
