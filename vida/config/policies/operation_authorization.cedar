permit (
  principal,
  action,
  resource
)
when {
  principal.claim_kind == resource.required_claim &&
  principal.capability == resource.required_capability &&
  principal.client_kind == resource.allowed_client_kind &&
  action.operation == resource.operation &&
  (
    resource.scope == "service" ||
    principal.project_id == resource.project_id
  ) &&
  (
    resource.owned_path == "" ||
    principal.owned_path == resource.owned_path
  )
};
