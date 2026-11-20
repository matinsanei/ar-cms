import { describe, it, expect, beforeAll } from 'vitest';

const BACKEND_URL = 'http://localhost:8088';

describe('Deep Edge Cases, Security & High-Concurrency Stress Suite', () => {
  let authToken = '';

  beforeAll(async () => {
    const loginRes = await fetch(`${BACKEND_URL}/admin/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email: 'matin@pixelstack.io', password: 'Password123!' }),
    });
    const loginData = await loginRes.json();
    authToken = loginData.data.token;
  });

  describe('1. Security Resiliency & Attack Payloads', () => {
    it('safely handles SQL injection attempts in query parameters without syntax error', async () => {
      const sqlInjectionQueries = [
        `filters[name][$eq]=' OR '1'='1`,
        `filters[name][$eq]=1; DROP TABLE cms_admins;--`,
        `filters[name][$contains]=UNION SELECT * FROM cms_tokens`,
      ];

      for (const q of sqlInjectionQueries) {
        const res = await fetch(`${BACKEND_URL}/api/products?${encodeURI(q)}`);
        expect(res.status).toBe(200);
        const data = await res.json();
        // Parametric bindings must ensure clean empty results rather than 500 or SQL leakage
        expect(Array.isArray(data.data)).toBe(true);
      }
    });

    it('safely rejects path traversal attempts in media resize filenames', async () => {
      const traversalPaths = [
        '../../../../etc/passwd',
        '..%2F..%2F..%2Fetc%2Fpasswd',
        '..\\..\\windows\\win.ini',
      ];

      for (const p of traversalPaths) {
        const res = await fetch(`${BACKEND_URL}/uploads/resize/${p}?w=50`);
        expect([400, 404]).toContain(res.status);
      }
    });

    it('handles malformed JSON request bodies gracefully without crash', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: '{"email": "broken_json...,,}',
      });
      expect([400, 422]).toContain(res.status);
    });

    it('returns standard 404 NotFoundError for unmapped route', async () => {
      const res = await fetch(`${BACKEND_URL}/non-existent-route-12345`);
      expect(res.status).toBe(404);
    });
  });

  describe('2. Trailing Slash Tolerance', () => {
    it('handles trailing slash uniformly on all admin endpoints', async () => {
      const endpoints = [
        '/admin/users/',
        '/admin/roles/',
        '/admin/webhooks/',
        '/admin/plugins/',
        '/admin/providers/',
        '/upload/files/',
        '/i18n/locales/',
      ];

      for (const ep of endpoints) {
        const res = await fetch(`${BACKEND_URL}${ep}`, {
          headers: { Authorization: `Bearer ${authToken}` },
        });
        expect(res.status).toBe(200);
      }
    });
  });

  describe('3. System Health & Diagnostics', () => {
    it('GET /admin/health returns 200 with engine metrics', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/health`);
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.status).toBe('healthy');
      expect(data.database.status).toBe('connected');
      expect(data.features.zeroRestartHotReload).toBe(true);
      expect(data.features.dynamicImageResizer).toBe(true);
      expect(data.features.geoSpatialQueries).toBe(true);
      expect(data.features.contentRevisions).toBe(true);
    });

    it('GET /_health returns 200 for container orchestrator probes', async () => {
      const res = await fetch(`${BACKEND_URL}/_health`);
      expect(res.status).toBe(200);
    });
  });

  describe('4. High-Concurrency Stress Test (50 Concurrent Async Requests)', () => {
    it('executes 50 simultaneous parallel read and query operations without locking or slowdown', async () => {
      const requests = Array.from({ length: 50 }, (_, i) => {
        if (i % 3 === 0) {
          return fetch(`${BACKEND_URL}/api/products?pageSize=5`);
        } else if (i % 3 === 1) {
          return fetch(`${BACKEND_URL}/api/products/geojson`);
        } else {
          return fetch(`${BACKEND_URL}/admin/health`);
        }
      });

      const start = Date.now();
      const responses = await Promise.all(requests);
      const elapsed = Date.now() - start;

      // All 50 concurrent requests must succeed
      for (const res of responses) {
        expect(res.status).toBe(200);
      }

      // SQLite WAL mode + Axum should complete 50 requests in well under 2 seconds
      expect(elapsed).toBeLessThan(3000);
    });
  });
});
