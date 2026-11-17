-- Healthcare: the database's own guard against double booking. A practitioner's visits still to
-- come or under way never overlap on a day, nor do a patient's: minutes past midnight, half-open,
-- so 09:00-09:20 and 09:20-09:40 do not. The appointment hook refuses a clash first, naming the
-- other visit; these hold when two bookings arrive at once (src/lib.rs registers their words).
alter table e_appointment add constraint healthcare_practitioner_free
  exclude using gist (tenant_id with =, practitioner with =, "date" with =, int8range(starts_min, ends_min) with &&)
  where (status in ('Scheduled', 'Checked In'));

alter table e_appointment add constraint healthcare_patient_free
  exclude using gist (tenant_id with =, patient with =, "date" with =, int8range(starts_min, ends_min) with &&)
  where (status in ('Scheduled', 'Checked In'));
