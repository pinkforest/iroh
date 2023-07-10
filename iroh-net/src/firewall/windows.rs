//! Windows firewall integration.
//!

/// Handle to apply rules using the Windows Filtering Platform (Fwpm).
#[derive(Debug)]
pub struct Firewall {
    session: Session,
    provider_id: GUID,
    sublayer_id: GUID,
}

const WEIGHT_IROH_TRAFFIC: u16 = 15;

impl Firewall {
    pub fn new() -> Result<Self> {
        let session = Session::new("Iroh firewall", "rules for iroh-net", true)?;
        let provider_id = GUID::new()?;
        session.add_provider(Provider::new(provider_id, "Iroh provider"))?;
        let sublayer_id = GUID::new()?;
        session.add_sublayer(Sublayer::new(
            sublayer_id,
            "Iroh permissive and blocking filters",
            0,
        ))?;

        let this = Firewall {
            session,
            provider_id,
            sublayer_id,
        };

        this.enable()?;
        Ok(this)
    }

    fn enable(&self) -> Result<()> {
        self.permit_iroh_service()?;
    }

    fn permit_iroh_serivce(&self) -> Result<()> {
        // TODO:

        Ok(())
    }

    fn permit_dns(&self) -> Result<()> {
        let conditions = [
            Match {
                field: FieldId::IpRemotePort,
                op: MatchType::Equal,
                value: MatchValue::U16(53),
            },
            // Repeat the condition type for logical OR.
            Match {
                field: FieldId::IpProtocol,
                op: MatchType::Equal,
                value: MatchValue::IpProtoUdp,
            },
            Match {
                field: FieldId::IpProtocol,
                op: MatchType::Equal,
                value: MatchValue::IpProtoTcp,
            },
        ];
        self.add_rules(
            "DNS",
            WEIGHT_IROH_TRAFFIC,
            conditions,
            Action::Permit,
            protocolAll,
            directionBoth,
        )?;
        Ok(())
    }

    // fn add_rules(&self, name: &str, )
}
