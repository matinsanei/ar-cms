import { describe, it, expect, beforeAll } from 'vitest';

const BACKEND_URL = 'http://127.0.0.1:8088';

describe('Zenith CMS / Strapi 5 Official API Contracts Suite', () => {
  let createdWebhookId = '';
  let createdApiTokenId = 0;
  let createdAdminTokenId = 0;
  let createdTransferTokenId = 0;
  let createdRoleId = 0;
  let createdEntryId = 0;
  let createdDocumentId = '';

  // ----------------------------------------------------
  // 1. Initial Setup & Admin Protocols
  // ----------------------------------------------------
  describe('Admin Setup & Auth Protocols', () => {
    it('GET /admin/init returns valid structure with hasAdmin', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/init`);
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data).toHaveProperty('data');
      expect(typeof data.data.hasAdmin).toBe('boolean');
      expect(data.data.uuid).toBe(false);
    });

    it('POST /admin/login handles authentication and returns token + sanitized user', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: 'matin@pixelstack.io', password: 'Password123!' }),
      });
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.token).toBeDefined();
      expect(body.data.user.email).toBe('matin@pixelstack.io');
      expect(body.data.user.roles[0].code).toBe('strapi-super-admin');
    });

    it('GET /admin/users/me returns currently authenticated admin profile', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/users/me`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.id).toBe(1);
      expect(body.data.email).toBe('matin@pixelstack.io');
      expect(body.data.isActive).toBe(true);
      expect(body.data.preferedLanguage).toBe('en');
    });

    it('GET /admin/users/me/permissions returns full permissions array', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/users/me/permissions`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(Array.isArray(body.data)).toBe(true);
      expect(body.data.length).toBeGreaterThan(20);
      const actions = body.data.map((p: any) => p.action);
      expect(actions).not.toContain('admin::marketplace.read');
      expect(actions).toContain('admin::webhooks.read');
      expect(actions).toContain('plugin::content-manager.explorer.read');
      expect(actions).toContain('plugin::content-type-builder.read');

      // Verify explicit subject permissions for content-manager models exist
      const articleReadPerm = body.data.find(
        (p: any) => p.action === 'plugin::content-manager.explorer.read' && p.subject === 'api::article.article'
      );
      expect(articleReadPerm).toBeDefined();
      expect(articleReadPerm.properties.locales).toContain('en');
      expect(Array.isArray(articleReadPerm.properties.fields)).toBe(true);
      expect(articleReadPerm.properties.fields.length).toBeGreaterThan(0);
    });

    it('POST /admin/permissions/check returns boolean evaluation', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/permissions/check`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ permissions: [{ action: 'admin::users.read' }] }),
      });
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data).toEqual([true]);
    });
  });

  // ----------------------------------------------------
  // 2. Settings & System Info
  // ----------------------------------------------------
  describe('Settings & System Information Protocols', () => {
    it('GET /admin/project-settings returns logo object without nested data', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/project-settings`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body).toHaveProperty('menuLogo');
      expect(body).toHaveProperty('authLogo');
    });

    it('GET /admin/information returns environment & dependency details', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/information`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.strapiVersion).toBe('1.0.0 (AR-CMS Engine)');
      expect(body.data.nodeVersion).toBe('Rust Axum + SQLx');
      expect(body.data.dependencies).toBeDefined();
      expect(body.data.communityEdition).toBe(true);
    });

    it('GET /admin/plugins returns loaded plugins list', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/plugins`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(Array.isArray(body.plugins)).toBe(true);
      const names = body.plugins.map((p: any) => p.name);
      expect(names).toContain('content-manager');
      expect(names).toContain('content-type-builder');
    });

    it('GET /admin/providers and SSO locked check', async () => {
      const ssoRes = await fetch(`${BACKEND_URL}/admin/providers/isSSOLocked`);
      expect(ssoRes.status).toBe(200);
      const ssoData = await ssoRes.json();
      expect(ssoData.data.isSSOLocked).toBe(false);

      const provRes = await fetch(`${BACKEND_URL}/admin/providers`);
      expect(provRes.status).toBe(200);
      const provData = await provRes.json();
      expect(Array.isArray(provData)).toBe(true);
    });

    it('GET /admin/permissions returns complete permissions catalog & sections', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/permissions`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.sections.collectionTypes).toBeDefined();
      expect(body.data.sections.plugins).toBeDefined();
      expect(body.data.sections.settings).toBeDefined();
      expect(body.data.sections.singleTypes).toBeDefined();
    });
  });

  // ----------------------------------------------------
  // 3. Webhooks Management
  // ----------------------------------------------------
  describe('Webhooks Management', () => {
    it('POST /admin/webhooks creates a new webhook', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/webhooks`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: 'Discord Webhook',
          url: 'https://discord.com/api/webhooks/123/xyz',
        }),
      });
      expect(res.status).toBe(201);
      const body = await res.json();
      expect(body.data.id).toBeDefined();
      expect(body.data.name).toBe('Discord Webhook');
      createdWebhookId = body.data.id;
    });

    it('GET /admin/webhooks/:id retrieves the created webhook', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/webhooks/${createdWebhookId}`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.id).toBe(createdWebhookId);
      expect(body.data.isEnabled).toBe(true);
    });

    it('POST /admin/webhooks/:id/trigger triggers test dispatch', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/webhooks/${createdWebhookId}/trigger`, {
        method: 'POST',
      });
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.statusCode).toBe(200);
    });

    it('DELETE /admin/webhooks/:id removes the webhook', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/webhooks/${createdWebhookId}`, {
        method: 'DELETE',
      });
      expect(res.status).toBe(200);
    });
  });

  // ----------------------------------------------------
  // 4. API & Transfer Tokens
  // ----------------------------------------------------
  describe('API & Transfer Tokens', () => {
    it('POST /admin/api-tokens creates content-api token', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/api-tokens`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: 'Mobile App Token',
          description: 'Access token for React Native app',
          type: 'read-only',
        }),
      });
      expect(res.status).toBe(201);
      const body = await res.json();
      expect(body.data.id).toBeDefined();
      expect(body.data.accessKey).toBeDefined();
      createdApiTokenId = body.data.id;
    });

    it('POST /admin/api-tokens/:id/regenerate refreshes key', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/api-tokens/${createdApiTokenId}/regenerate`, {
        method: 'POST',
      });
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.accessKey).toBeDefined();
    });

    it('POST /admin/admin-tokens creates an admin token', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/admin-tokens`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: 'DevOps Token',
          description: 'Token for CI/CD pipelines',
        }),
      });
      expect(res.status).toBe(201);
      const body = await res.json();
      expect(body.data.id).toBeDefined();
      createdAdminTokenId = body.data.id;
    });

    it('POST /admin/transfer/tokens creates a data transfer token', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/transfer/tokens`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: 'Staging Sync',
          description: 'Transfer data from staging',
        }),
      });
      expect(res.status).toBe(201);
      const body = await res.json();
      expect(body.data.accessKey).toBeDefined();
      expect(body.data.permissions).toEqual(['push', 'pull']);
      createdTransferTokenId = body.data.id;
    });
  });

  // ----------------------------------------------------
  // 5. Roles & Users Management
  // ----------------------------------------------------
  describe('Roles & Users Management', () => {
    it('GET /admin/roles lists built-in and custom roles', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/roles`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(Array.isArray(body.data)).toBe(true);
      expect(body.data.length).toBeGreaterThanOrEqual(3);
    });

    it('POST /admin/roles creates custom role', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/roles`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: 'Blog Moderator',
          description: 'Can manage comments and draft posts',
        }),
      });
      expect(res.status).toBe(201);
      const body = await res.json();
      expect(body.data.name).toBe('Blog Moderator');
      createdRoleId = body.data.id;
    });

    it('GET /admin/roles/:id/permissions returns role permissions', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/roles/${createdRoleId}/permissions`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(Array.isArray(body.data)).toBe(true);
    });

    it('GET /admin/users returns paginated admin user list', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/users`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.results).toBeDefined();
      expect(body.data.pagination.total).toBeGreaterThanOrEqual(1);
    });
  });

  // ----------------------------------------------------
  // 6. Content-Type Builder & Schema Hot-Reload
  // ----------------------------------------------------
  describe('Content-Type Builder & Hot-Reload', () => {
    it('GET /content-type-builder/schema returns all content-types with attributes as array', async () => {
      const res = await fetch(`${BACKEND_URL}/content-type-builder/schema`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.contentTypes).toBeDefined();
      expect(body.data.contentTypes['api::product.product']).toBeDefined();
      expect(Array.isArray(body.data.contentTypes['api::product.product'].attributes)).toBe(true);
      expect(body.data.contentTypes['api::product.product'].attributes[0]).toHaveProperty('name');
      expect(body.data.contentTypes['api::product.product'].modelType).toBe('contentType');
    });

    it('GET /content-type-builder/reserved-names returns models and attributes arrays', async () => {
      const res = await fetch(`${BACKEND_URL}/content-type-builder/reserved-names`);
      expect(res.status).toBe(200);
      const body = await res.json();
      // Verifies compatibility with Strapi 5 CTB validation (FormModal Yup schema)
      const reservedAttrs = body.attributes || body.data?.reservedNames?.attributes || body.data?.attributes;
      const reservedModels = body.models || body.data?.reservedNames?.models || body.data?.models;
      expect(Array.isArray(reservedAttrs)).toBe(true);
      expect(reservedAttrs).toContain('id');
      expect(reservedAttrs).toContain('document_id');
      expect(Array.isArray(reservedModels)).toBe(true);
      expect(reservedModels).toContain('boolean');
    });

    it('GET /content-type-builder/update-schema-status indicates 0-wait hot reload', async () => {
      const res = await fetch(`${BACKEND_URL}/content-type-builder/update-schema-status`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.isUpdating).toBe(false);
    });

    it('POST /content-type-builder/content-types creates a new schema with instant hot-reload', async () => {
      const newSchema = {
        contentType: {
          uid: 'api::article.article',
          kind: 'collectionType',
          info: {
            singularName: 'article',
            pluralName: 'articles',
            displayName: 'Article',
            description: 'News articles and blog posts',
          },
          options: { draftAndPublish: true },
          attributes: {
            title: { type: 'string', required: true, unique: false },
            content: { type: 'text', required: false, unique: false },
          },
        },
      };

      const res = await fetch(`${BACKEND_URL}/content-type-builder/content-types`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(newSchema),
      });
      expect(res.status).toBe(200);

      // Verify immediate availability in GET /content-type-builder/schema
      const verifyRes = await fetch(`${BACKEND_URL}/content-type-builder/schema`);
      const verifyBody = await verifyRes.json();
      expect(verifyBody.data.contentTypes['api::article.article']).toBeDefined();
    });
  });

  // ----------------------------------------------------
  // 7. Content Manager & Collection-Types CRUD
  // ----------------------------------------------------
  describe('Content Manager & Collection-Types CRUD', () => {
    it('GET /content-manager/init returns initialization bundle for content manager', async () => {
      const res = await fetch(`${BACKEND_URL}/content-manager/init`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data).toHaveProperty('contentTypes');
      expect(body.data).toHaveProperty('components');
    });

    it('GET /content-manager/content-types returns schema collection for content manager', async () => {
      const res = await fetch(`${BACKEND_URL}/content-manager/content-types`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(Array.isArray(body.data)).toBe(true);
    });

    it('GET /content-manager/content-types-settings returns list configurations', async () => {
      const res = await fetch(`${BACKEND_URL}/content-manager/content-types-settings`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(Array.isArray(body.data)).toBe(true);
    });

    it('POST /content-manager/collection-types/:uid creates an entry', async () => {
      const res = await fetch(`${BACKEND_URL}/content-manager/collection-types/api::product.product`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: 'Mechanical Keyboard RGB',
          price: 149.99,
          badge: 'Hot Deal',
        }),
      });
      expect(res.status).toBe(201);
      const body = await res.json();
      expect(body.data.id).toBeDefined();
      expect(body.data.documentId).toBeDefined();
      expect(body.data.name).toBe('Mechanical Keyboard RGB');
      expect(body.data.publishedAt).toBeNull();
      expect(body.data.status).toBe('draft');
      expect(body.meta).toBeDefined();
      expect(body.meta.defaultLocale).toBe('en');
      expect(Array.isArray(body.meta.availableStatus)).toBe(true);
      createdEntryId = body.data.id;
      createdDocumentId = body.data.documentId;
    });

    it('GET /content-manager/collection-types/:uid/:documentId retrieves entry by UUID with Strapi 5 meta', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${createdDocumentId}`
      );
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.documentId).toBe(createdDocumentId);
      expect(body.data.name).toBe('Mechanical Keyboard RGB');
      expect(body.meta).toBeDefined();
      expect(body.meta.availableStatus[0].documentId).toBe(createdDocumentId);
      expect(body.meta.availableStatus[0].status).toBe('draft');
      expect(body.meta.defaultLocale).toBe('en');
    });

    it('PUT /content-manager/collection-types/:uid/:documentId updates entry by UUID', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${createdDocumentId}`,
        {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            name: 'Mechanical Keyboard RGB Pro',
            price: 179.99,
          }),
        }
      );
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.name).toBe('Mechanical Keyboard RGB Pro');
      expect(body.data.price).toBe(179.99);
      expect(body.data.documentId).toBe(createdDocumentId);
    });

    it('POST /content-manager/collection-types/:uid/:documentId/actions/publish publishes entry by UUID', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${createdDocumentId}/actions/publish`,
        { method: 'POST' }
      );
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.publishedAt).not.toBeNull();
      expect(body.data.status).toBe('published');
      expect(body.meta.availableStatus[0].status).toBe('published');
    });

    it('GET /api/products returns published entry via storefront API', async () => {
      const res = await fetch(`${BACKEND_URL}/api/products`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.length).toBeGreaterThanOrEqual(1);
      const found = body.data.find((p: any) => p.documentId === createdDocumentId);
      expect(found).toBeDefined();
      expect(found.name).toBe('Mechanical Keyboard RGB Pro');
    });

    it('POST /content-manager/collection-types/:uid/:documentId/actions/unpublish unpublishes entry by UUID', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${createdDocumentId}/actions/unpublish`,
        { method: 'POST' }
      );
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.publishedAt).toBeNull();
      expect(body.data.status).toBe('draft');
      expect(body.meta.availableStatus[0].status).toBe('draft');
    });

    it('GET /content-manager/collection-types/:uid/:randomUuid auto-initializes draft stub for new page navigation', async () => {
      const randomUuid = 'c28dd9c0-aebb-4160-92f1-5285bbdea999';
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::page.page/${randomUuid}`
      );
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.documentId).toBe(randomUuid);
      expect(body.data.status).toBe('draft');
      expect(body.meta.availableStatus[0].documentId).toBe(randomUuid);
    });

    it('GET /content-manager/collection-types/:uid/:documentId/actions/countDraftRelations returns 0 relations without error', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${createdDocumentId}/actions/countDraftRelations`
      );
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data).toHaveProperty('unpublishedRelations');
      expect(body.data).toHaveProperty('draftM2mLinks');
      expect(body.data.unpublishedRelations).toBe(0);
      expect(body.data.draftM2mLinks).toBe(0);
    });

    it('GET /content-manager/preview/url/:contentType returns null url without 404', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/preview/url/api::page.page?documentId=${createdDocumentId}&locale=en&status=published`
      );
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data).toHaveProperty('url');
    });

    it('POST bulkPublish and bulkUnpublish with documentIds array', async () => {
      const resPub = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/actions/bulkPublish`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ documentIds: [createdDocumentId] }),
        }
      );
      expect(resPub.status).toBe(200);
      const pubBody = await resPub.json();
      expect(pubBody.data.count).toBe(1);

      const resUnpub = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/actions/bulkUnpublish`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ documentIds: [createdDocumentId] }),
        }
      );
      expect(resUnpub.status).toBe(200);
      const unpubBody = await resUnpub.json();
      expect(unpubBody.data.count).toBe(1);
    });

    it('DELETE /content-manager/collection-types/:uid/:documentId deletes the entry by UUID', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${createdDocumentId}`,
        { method: 'DELETE' }
      );
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.documentId).toBe(createdDocumentId);
    });
  });

  // ----------------------------------------------------
  // 8. Media Library & i18n
  // ----------------------------------------------------
  describe('Media Library & i18n Protocols', () => {
    it('GET /content-manager/homepage/count-documents returns document status counts', async () => {
      const res = await fetch(`${BACKEND_URL}/content-manager/homepage/count-documents`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data).toHaveProperty('draft');
      expect(body.data).toHaveProperty('published');
    });

    it('GET /admin/homepage/key-statistics returns live aggregate metrics', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/homepage/key-statistics`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data).toHaveProperty('admins');
      expect(body.data).toHaveProperty('contentTypes');
      expect(body.data).toHaveProperty('webhooks');
    });

    it('GET /upload/settings & /upload/configuration return media settings', async () => {
      const res = await fetch(`${BACKEND_URL}/upload/settings`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data.sizeOptimization).toBe(true);

      const configRes = await fetch(`${BACKEND_URL}/upload/configuration`);
      expect(configRes.status).toBe(200);
      const configBody = await configRes.json();
      expect(configBody.data.pageSize).toBe(10);
    });

    it('GET /i18n/locales returns configured locales', async () => {
      const res = await fetch(`${BACKEND_URL}/i18n/locales`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(Array.isArray(body)).toBe(true);
      expect(body[0].code).toBe('en');
    });
  });

  // ----------------------------------------------------
  // 7. Trailing Slash & Query Parameters Normalization
  // ----------------------------------------------------
  describe('Trailing Slash & Query Params Contract Handling', () => {
    it('GET /admin/webhooks/ responds with 200 OK', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/webhooks/`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(Array.isArray(body.data)).toBe(true);
    });

    it('GET /admin/roles/ responds with 200 OK', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/roles/`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(Array.isArray(body.data)).toBe(true);
    });

    it('GET /admin/users/?pageSize=10&page=1&sort=firstname responds with 200 OK', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/users/?pageSize=10&page=1&sort=firstname`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.data).toHaveProperty('results');
      expect(body.data).toHaveProperty('pagination');
    });

    it('GET /content-manager/content-types/ responds with 200 OK', async () => {
      const res = await fetch(`${BACKEND_URL}/content-manager/content-types/`);
      expect(res.status).toBe(200);
    });

    it('GET /content-manager/content-types-settings/ responds with 200 OK', async () => {
      const res = await fetch(`${BACKEND_URL}/content-manager/content-types-settings/`);
      expect(res.status).toBe(200);
    });

    it('GET /upload/files/ responds with 200 OK', async () => {
      const res = await fetch(`${BACKEND_URL}/upload/files/`);
      expect(res.status).toBe(200);
    });

    it('GET /i18n/locales/ responds with 200 OK', async () => {
      const res = await fetch(`${BACKEND_URL}/i18n/locales/`);
      expect(res.status).toBe(200);
    });
  });
});
