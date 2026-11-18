export type FieldType = 
  | 'string'
  | 'text'
  | 'richtext'
  | 'integer'
  | 'biginteger'
  | 'float'
  | 'decimal'
  | 'boolean'
  | 'date'
  | 'datetime'
  | 'json'
  | 'media'
  | 'relation'
  | 'component'
  | 'dynamiczone';

export interface AttributeDefinition {
  type: FieldType;
  required?: boolean;
  unique?: boolean;
  default?: any;
  minLength?: number;
  maxLength?: number;
  configurable?: boolean;
  target?: string; // For relations
  component?: string; // For components
  components?: string[]; // For dynamic zones
  multiple?: boolean;
}

export interface ContentTypeSchema {
  uid: string;
  kind: 'collectionType' | 'singleType';
  info: {
    singularName: string;
    pluralName: string;
    displayName: string;
    description?: string;
  };
  options?: {
    draftAndPublish?: boolean;
  };
  attributes: Record<string, AttributeDefinition>;
}

export interface ContentEntry {
  id: string | number;
  documentId?: string;
  createdAt: string;
  updatedAt: string;
  publishedAt?: string | null;
  [key: string]: any;
}
