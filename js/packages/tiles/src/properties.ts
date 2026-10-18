// Reads custom properties: the `properties` array of a map, layer, object or tile. Each property has a
// name, a type (a string when the type is left out) and a value. The value must match its type. A class
// property keeps its members; the members are typed by their JSON type, since the map does not carry the
// class definition from the project.

import { Fields, fieldPath, isRecord, itemPath, Problems } from "./reader";
import type { TileProperties, TileProperty } from "./types";

/** A colour as Tiled writes it: `#RRGGBB` or `#AARRGGBB`, or "" when the colour is not set. */
export const COLOUR_PATTERN = /^(?:#(?:[0-9a-fA-F]{6}|[0-9a-fA-F]{8}))?$/;

/** The property types that this reader knows. Any other name is refused as unsupported. */
const KNOWN_TYPES = new Set(["string", "int", "float", "bool", "color", "file", "object", "class"]);

/**
 * Reads a `properties` array.
 * @param problems - where the problems go.
 * @param raw - the JSON value, which may be undefined when the array is left out.
 * @param path - the path of the array in the document.
 * @returns the properties by name. Empty when the array is missing or broken.
 */
export function readProperties(problems: Problems, raw: unknown, path: string): TileProperties {
  if (raw === undefined) return {};
  if (!Array.isArray(raw)) {
    problems.add("structure", "properties must be an array", path);
    return {};
  }
  const found = new Map<string, TileProperty>();
  raw.forEach((item, index) => {
    const itemAt = itemPath(path, index);
    if (!isRecord(item)) {
      problems.add("structure", "a property must be an object", itemAt);
      return;
    }
    const fields = new Fields(problems, item, itemAt);
    const name = fields.string("name");
    if (name === "") fields.fail("value", "a property needs a name", "name");
    const property = readProperty(fields);
    if (property === undefined) return;
    if (found.has(name)) {
      fields.fail("duplicate", `the property "${name}" is set twice`, "name");
      return;
    }
    found.set(name, property);
  });
  return Object.fromEntries(found);
}

/** Reads one typed property: its type, then its value. Undefined when the type is not one this reader knows. */
function readProperty(fields: Fields): TileProperty | undefined {
  const type = fields.has("type") ? fields.string("type") : "string";
  if (!KNOWN_TYPES.has(type)) {
    fields.fail("unsupported", `the property type "${type}" is not supported`, "type");
    return undefined;
  }
  switch (type) {
    case "string":
      return { type, value: fields.string("value", "") };
    case "int":
      return { type, value: fields.int("value", { default: 0 }) };
    case "float":
      return { type, value: fields.number("value", { default: 0 }) };
    case "bool":
      return { type, value: fields.bool("value", false) };
    case "color": {
      const value = fields.string("value", "");
      if (!COLOUR_PATTERN.test(value)) {
        fields.fail("value", `"${value}" is not a colour: use #RRGGBB or #AARRGGBB`, "value");
      }
      return { type, value };
    }
    case "file":
      return { type, value: fields.string("value", "") };
    case "object":
      return { type, value: fields.int("value", { min: 0, default: 0 }) };
    default:
      return readClass(fields);
  }
}

/** Reads a class property: the class name, then its members. */
function readClass(fields: Fields): TileProperty {
  const propertyType = fields.string("propertytype", "");
  const members = fields.raw("value");
  if (members === undefined) {
    fields.fail("structure", '"value" is required', "value");
    return { type: "class", propertyType, value: {} };
  }
  if (!isRecord(members)) {
    fields.fail("structure", "a class value must be an object of members", "value");
    return { type: "class", propertyType, value: {} };
  }
  return { type: "class", propertyType, value: readMembers(fields.recorder, members, fields.at("value")) };
}

/**
 * Reads the members of a class value. A member is typed by its JSON type: a string, a bool, a whole number
 * (int), any other number (float), or an object (a nested class with no class name).
 */
function readMembers(problems: Problems, members: Record<string, unknown>, path: string): TileProperties {
  // A Map keeps names such as "__proto__" as plain entries.
  const found = new Map<string, TileProperty>();
  for (const [name, value] of Object.entries(members)) {
    const memberAt = fieldPath(path, name);
    if (typeof value === "string") {
      found.set(name, { type: "string", value });
    } else if (typeof value === "boolean") {
      found.set(name, { type: "bool", value });
    } else if (typeof value === "number") {
      found.set(name, Number.isInteger(value) ? { type: "int", value } : { type: "float", value });
    } else if (isRecord(value)) {
      found.set(name, { type: "class", propertyType: "", value: readMembers(problems, value, memberAt) });
    } else {
      problems.add("structure", `the member "${name}" must be a string, bool, number or object`, memberAt);
    }
  }
  return Object.fromEntries(found);
}
