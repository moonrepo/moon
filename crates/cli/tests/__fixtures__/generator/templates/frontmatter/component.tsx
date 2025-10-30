{% set component_name = "small-button" | pascal_case %}

---
to: components/{{ component_name }}.tsx
---

export function {{ component_name }}() {
	return null;
}
