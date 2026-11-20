import { describe, it, expect, beforeAll } from 'vitest';

const BACKEND_URL = 'http://localhost:8088';

describe('Deep Content Manager & Schema Hot-Reload Suite', () => {
  let authToken = '';
  const customSchemaUid = 'api::article.article';
  let createdArticleDocId = '';
  const bulkArticleIds: string[] = [];

  beforeAll(async () => {
    const loginRes = await fetch(`${BACKEND_URL}/admin/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email: 'matin@pixelstack.io', password: 'Password123!' }),
    });
    const loginData = await loginRes.json();
    authToken = loginData.data.token;
  });

  describe('1. Dynamic Schema Creation & Instant Hot-Reload', () => {
    it('creates a new custom Article collection schema without server restart', async () => {
      const res = await fetch(`${BACKEND_URL}/content-type-builder/content-types`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`,
        },
        body: JSON.stringify({
          contentType: {
            uid: customSchemaUid,
            kind: 'collectionType',
            info: {
              singularName: 'article',
              pluralName: 'articles',
              displayName: 'Article',
            },
            options: { draftAndPublish: true },
            attributes: {
              title: { type: 'string', required: true },
              slug: { type: 'string', unique: true },
              content: { type: 'text' },
              views: { type: 'integer', default: 0 },
              isFeatured: { type: 'boolean', default: false },
            },
          },
        }),
      });

      expect([200, 201]).toContain(res.status);
      const data = await res.json();
      expect(data.data.uid).toBe(customSchemaUid);
    });

    it('immediately reads the newly created schema via /content-type-builder/content-types/:uid', async () => {
      const res = await fetch(`${BACKEND_URL}/content-type-builder/content-types/${customSchemaUid}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.uid).toBe(customSchemaUid);
      expect(data.data.info.pluralName).toBe('articles');
    });

    it('verifies update-schema-status indicates 0-wait hot reload', async () => {
      const res = await fetch(`${BACKEND_URL}/content-type-builder/update-schema-status`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.isUpdating).toBe(false);
    });

    it('verifies new schema is registered in Content Manager schemas catalog', async () => {
      const res = await fetch(`${BACKEND_URL}/content-manager/content-types`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      const articleModel = data.data.find((m: any) => m.uid === customSchemaUid);
      expect(articleModel).toBeDefined();
      expect(articleModel.info.displayName).toBe('Article');
    });
  });

  describe('2. Comprehensive Content Manager Entry Lifecycle', () => {
    it('creates draft entry with default metadata (status=draft, locale=en)', async () => {
      const res = await fetch(`${BACKEND_URL}/content-manager/collection-types/${customSchemaUid}`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`,
        },
        body: JSON.stringify({
          title: 'Deep Dive into Rust Axum Web Engine',
          slug: 'rust-axum-web-engine',
          content: 'Axum provides high concurrency with Tokio...',
          views: 1500,
          isFeatured: true,
        }),
      });

      expect([200, 201]).toContain(res.status);
      const data = await res.json();
      expect(data.data.id).toBeDefined();
      expect(data.data.documentId).toBeDefined();
      expect(data.data.title).toBe('Deep Dive into Rust Axum Web Engine');
      expect(data.data.status).toBe('draft');
      expect(data.data.locale).toBe('en');
      createdArticleDocId = data.data.documentId;
    });

    it('retrieves created entry by documentId', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/${customSchemaUid}/${createdArticleDocId}`,
        {
          headers: { Authorization: `Bearer ${authToken}` },
        }
      );
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.documentId).toBe(createdArticleDocId);
      expect(data.data.slug).toBe('rust-axum-web-engine');
    });

    it('auto-heals draft stub when navigating to new random UUID', async () => {
      const randomUuid = 'random-draft-uuid-' + Date.now();
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/${customSchemaUid}/${randomUuid}`,
        {
          headers: { Authorization: `Bearer ${authToken}` },
        }
      );
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.documentId).toBe(randomUuid);
      expect(data.data.status).toBe('draft');
    });

    it('updates entry fields partially without mutating other attributes', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/${customSchemaUid}/${createdArticleDocId}`,
        {
          method: 'PUT',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${authToken}`,
          },
          body: JSON.stringify({
            views: 3200,
          }),
        }
      );
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.views).toBe(3200);
      expect(data.data.title).toBe('Deep Dive into Rust Axum Web Engine'); // Preserved
    });

    it('verifies unpublished entry is NOT accessible via public storefront API', async () => {
      const res = await fetch(`${BACKEND_URL}/api/articles/${createdArticleDocId}`);
      expect(res.status).toBe(404);
    });

    it('publishes entry and updates publication timestamp & status', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/${customSchemaUid}/${createdArticleDocId}/actions/publish`,
        {
          method: 'POST',
          headers: { Authorization: `Bearer ${authToken}` },
        }
      );
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.status).toBe('published');
      expect(data.data.publishedAt).toBeDefined();
    });

    it('verifies published entry IS accessible via storefront API with formatted response', async () => {
      const res = await fetch(`${BACKEND_URL}/api/articles/${createdArticleDocId}`);
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.documentId).toBe(createdArticleDocId);
      expect(data.data.title).toBe('Deep Dive into Rust Axum Web Engine');

      // Also via collection list
      const listRes = await fetch(`${BACKEND_URL}/api/articles`);
      expect(listRes.status).toBe(200);
      const listData = await listRes.json();
      const found = listData.data.find((a: any) => a.documentId === createdArticleDocId);
      expect(found).toBeDefined();
    });

    it('unpublishes entry and revokes public storefront access', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/${customSchemaUid}/${createdArticleDocId}/actions/unpublish`,
        {
          method: 'POST',
          headers: { Authorization: `Bearer ${authToken}` },
        }
      );
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.status).toBe('draft');
      expect(data.data.publishedAt).toBeNull();

      // Verify immediate revocation on storefront
      const publicRes = await fetch(`${BACKEND_URL}/api/articles/${createdArticleDocId}`);
      expect(publicRes.status).toBe(404);
    });

    it('GET countDraftRelations returns 0 relations count', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/${customSchemaUid}/${createdArticleDocId}/actions/countDraftRelations`,
        {
          headers: { Authorization: `Bearer ${authToken}` },
        }
      );
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.unpublishedRelations).toBe(0);
      expect(data.data.draftM2mLinks).toBe(0);
    });
  });

  describe('3. Bulk Operations (Publish, Unpublish, Delete)', () => {
    it('creates 5 articles in draft mode for bulk testing', async () => {
      for (let i = 1; i <= 5; i++) {
        const res = await fetch(`${BACKEND_URL}/content-manager/collection-types/${customSchemaUid}`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${authToken}`,
          },
          body: JSON.stringify({
            title: `Bulk Test Article ${i}`,
            slug: `bulk-article-${i}-${Date.now()}`,
            views: i * 10,
          }),
        });
        const d = await res.json();
        bulkArticleIds.push(d.data.documentId);
      }
      expect(bulkArticleIds.length).toBe(5);
    });

    it('bulk publishes all 5 articles simultaneously', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/${customSchemaUid}/actions/bulkPublish`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${authToken}`,
          },
          body: JSON.stringify({
            documentIds: bulkArticleIds,
          }),
        }
      );
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.count).toBe(5);
    });

    it('bulk unpublishes all 5 articles simultaneously', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/${customSchemaUid}/actions/bulkUnpublish`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${authToken}`,
          },
          body: JSON.stringify({
            documentIds: bulkArticleIds,
          }),
        }
      );
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.count).toBe(5);
    });

    it('bulk deletes all 5 articles', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/${customSchemaUid}/actions/bulkDelete`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${authToken}`,
          },
          body: JSON.stringify({
            documentIds: bulkArticleIds,
          }),
        }
      );
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.count).toBe(5);
    });
  });

  describe('4. Single Types & Administration Statistics', () => {
    it('GET and PUT single type homepage settings', async () => {
      const putRes = await fetch(`${BACKEND_URL}/content-manager/single-types/api::homepage.homepage`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`,
        },
        body: JSON.stringify({
          heroTitle: 'Welcome to PixelStack Cloud',
          seoDescription: 'High performance enterprise headless architecture',
        }),
      });
      expect(putRes.status).toBe(200);

      const getRes = await fetch(`${BACKEND_URL}/content-manager/single-types/api::homepage.homepage`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      expect(getRes.status).toBe(200);
      const data = await getRes.json();
      expect(data.data.heroTitle).toBe('Welcome to PixelStack Cloud');
    });

    it('GET /content-manager/homepage/count-documents reports document distribution', async () => {
      const res = await fetch(`${BACKEND_URL}/content-manager/homepage/count-documents`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data).toBeDefined();
      expect(typeof data.data.draft).toBe('number');
      expect(typeof data.data.published).toBe('number');
    });
  });
});
