use rstd::prelude::*;
use parity_codec::Codec;
use parity_codec_derive::{Encode, Decode};
use srml_support::{StorageMap, StorageValue, dispatch, decl_module, decl_storage, decl_event, ensure, Parameter};
use runtime_primitives::traits::{SimpleArithmetic, As, Member, MaybeDebug, MaybeSerializeDebug};
use system::{self, ensure_signed};
use {timestamp};

pub trait Trait: system::Trait + timestamp::Trait + MaybeDebug {

  type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

  type BlogId: Parameter + Member + SimpleArithmetic + Codec + Default + Copy
    + As<usize> + As<u64> + MaybeSerializeDebug + PartialEq;

  type PostId: Parameter + Member + SimpleArithmetic + Codec + Default + Copy
    + As<usize> + As<u64> + MaybeSerializeDebug + PartialEq;

  type CommentId: Parameter + Member + SimpleArithmetic + Codec + Default + Copy
    + As<usize> + As<u64> + MaybeSerializeDebug + PartialEq;
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq)]
pub struct Change<T: Trait> {
  account: T::AccountId,
  block: T::BlockNumber,
  time: T::Moment,
}

// TODO add a schema along w/ JSON, maybe create a struct?

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq)]
pub struct Blog<T: Trait> {
  id: T::BlogId,
  created: Change<T>,
  updated: Option<Change<T>>,

  // Can be updated by the owner:
  writers: Vec<T::AccountId>,
  slug: Vec<u8>,
  json: Vec<u8>,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq)]
pub struct BlogUpdate<T: Trait> {
  writers: Option<Vec<T::AccountId>>,
  slug: Option<Vec<u8>>,
  json: Option<Vec<u8>>,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq)]
pub struct Post<T: Trait> {
  id: T::PostId,
  blog_id: T::BlogId,
  created: Change<T>,
  updated: Option<Change<T>>,

  // Next fields can be updated by the owner only:

  // TODO make slug optional for post or even remove it
  slug: Vec<u8>,
  json: Vec<u8>,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq)]
pub struct PostUpdate<T: Trait> {
  blog_id: Option<T::BlogId>,
  slug: Option<Vec<u8>>,
  json: Option<Vec<u8>>,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq)]
pub struct Comment<T: Trait> {
  id: T::CommentId,
  parent_id: Option<T::CommentId>,
  post_id: T::PostId,
  blog_id: T::BlogId,
  created: Change<T>,
  updated: Option<Change<T>>,

  // Can be updated by the owner:
  json: Vec<u8>,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq)]
pub struct CommentUpdate {
  json: Vec<u8>,
}

const DEFAULT_SLUG_MIN_LEN: u32 = 5;
const DEFAULT_SLUG_MAX_LEN: u32 = 50;

const DEFAULT_BLOG_MAX_LEN: u32 = 1_000;
const DEFAULT_POST_MAX_LEN: u32 = 10_000;
const DEFAULT_COMMENT_MAX_LEN: u32 = 1_000;

decl_storage! {
  trait Store for Module<T: Trait> as Blogs {

    SlugMinLen get(slug_min_len): u32 = DEFAULT_SLUG_MIN_LEN;
    SlugMaxLen get(slug_max_len): u32 = DEFAULT_SLUG_MAX_LEN;

    BlogMaxLen get(blog_max_len): u32 = DEFAULT_BLOG_MAX_LEN;
    PostMaxLen get(post_max_len): u32 = DEFAULT_POST_MAX_LEN;
    CommentMaxLen get(comment_max_len): u32 = DEFAULT_COMMENT_MAX_LEN;

    BlogById get(blog_by_id): map T::BlogId => Option<Blog<T>>;
    PostById get(post_by_id): map T::PostId => Option<Post<T>>;
    CommentById get(comment_by_id): map T::CommentId => Option<Comment<T>>;

    BlogIdsByOwner get(blog_ids_by_owner): map T::AccountId => Vec<T::BlogId>;
    PostIdsByBlogId get(post_ids_by_blog_id): map T::BlogId => Vec<T::PostId>;
    CommentIdsByPostId get(comment_ids_by_post_id): map T::PostId => Vec<T::CommentId>;

    BlogIdBySlug get(blog_id_by_slug): map Vec<u8> => Option<T::BlogId>;
    PostIdBySlug get(post_id_by_slug): map Vec<u8> => Option<T::PostId>;

    NextBlogId get(next_blog_id): T::BlogId = T::BlogId::sa(1);
    NextPostId get(next_post_id): T::PostId = T::PostId::sa(1);
    NextCommentId get(next_comment_id): T::CommentId = T::CommentId::sa(1);
  }
}

decl_event! {
  pub enum Event<T> where
    <T as system::Trait>::AccountId,
    <T as Trait>::BlogId,
    <T as Trait>::PostId,
    <T as Trait>::CommentId
  {
    BlogCreated(AccountId, BlogId),
    BlogUpdated(AccountId, BlogId),
    BlogDeleted(AccountId, BlogId),

    PostCreated(AccountId, PostId),
    PostUpdated(AccountId, PostId),
    PostDeleted(AccountId, PostId),

    CommentCreated(AccountId, CommentId),
    CommentUpdated(AccountId, CommentId),
    CommentDeleted(AccountId, CommentId),
  }
}

decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {

    fn deposit_event<T>() = default;

    fn on_initialize(_now: T::BlockNumber) {
      // Stub
    }

    fn on_finalize(_now: T::BlockNumber) {
      // Stub
    }

    fn create_blog(origin, data: BlogUpdate<T>) {
      let owner = ensure_signed(origin)?;

      ensure!(data.writers == None, "Blog writers shouldn't be provided on creation");

      let slug = data.slug.expect("Blog slug should be provided");
      ensure!(slug.len() >= Self::slug_min_len() as usize, "Blog slug is too short");
      ensure!(slug.len() <= Self::slug_max_len() as usize, "Blog slug is too long");
      ensure!(!<BlogIdBySlug<T>>::exists(slug.clone()), "Blog slug is not unique");

      let json = data.json.expect("Blog JSON should be provided");
      ensure!(json.len() <= Self::blog_max_len() as usize, "Blog JSON is too long");

      let blog_id = Self::next_blog_id();

      let new_blog: Blog<T> = Blog {
        id: blog_id,
        created: Self::new_change(owner.clone()),
        updated: None,
        writers: vec![],
        slug: slug.clone(),
        json
      };

      <BlogById<T>>::insert(blog_id, new_blog);
      <BlogIdsByOwner<T>>::mutate(owner.clone(), |ids| ids.push(blog_id));
      <BlogIdBySlug<T>>::insert(slug, blog_id);
      <NextBlogId<T>>::mutate(|n| { *n += T::BlogId::sa(1); });
      Self::deposit_event(RawEvent::BlogCreated(owner.clone(), blog_id));
    }

    fn create_post(origin, data: PostUpdate<T>) {
      let owner = ensure_signed(origin)?;

      let blog_id = data.blog_id.expect("Blog id should be provided");
      ensure!(<BlogById<T>>::exists(blog_id), "Unknown blog id");

      let slug = data.slug.expect("Post slug should be provided");
      ensure!(slug.len() >= Self::slug_min_len() as usize, "Post slug is too short");
      ensure!(slug.len() <= Self::slug_max_len() as usize, "Post slug is too long");
      ensure!(!<PostIdBySlug<T>>::exists(slug.clone()), "Post slug is not unique");

      let json = data.json.expect("Post json should be provided");
      ensure!(json.len() <= Self::post_max_len() as usize, "Post JSON is too long");
      // Todo: Ensure that post have some data in json

      let post_id = Self::next_post_id();

      let new_post: Post<T> = Post {
        id: post_id,
        blog_id,
        created: Self::new_change(owner.clone()),
        updated: None,
        slug: slug.clone(),
        json
      };

      <PostById<T>>::insert(post_id, new_post);
      <PostIdsByBlogId<T>>::mutate(blog_id, |ids| ids.push(post_id));
      <PostIdBySlug<T>>::insert(slug, post_id);
      <NextPostId<T>>::mutate(|n| { *n += T::PostId::sa(1); });
      Self::deposit_event(RawEvent::PostCreated(owner.clone(), post_id));
    }

    fn create_comment(origin, post_id: T::PostId, parent_id: Option<T::CommentId>, data: CommentUpdate) {
      let owner = ensure_signed(origin)?;

      ensure!(<PostById<T>>::exists(post_id), "Unknown post id");
      let post = Self::post_by_id(&post_id).unwrap();
      let blog_id = post.blog_id;

      if let Some(id) = parent_id {
        ensure!(<CommentById<T>>::exists(id), "Unknown parent comment id");
      }

      ensure!(data.json.len() <= Self::comment_max_len() as usize, "Comment JSON is too long");

      let comment_id = Self::next_comment_id();

      let new_comment: Comment<T> = Comment {
        id: comment_id,
        parent_id,
        post_id,
        blog_id,
        created: Self::new_change(owner.clone()),
        updated: None,
        json: data.json,
      };

      <CommentById<T>>::insert(comment_id, new_comment);
      <CommentIdsByPostId<T>>::mutate(post_id, |ids| ids.push(comment_id));
      <NextCommentId<T>>::mutate(|n| { *n += T::CommentId::sa(1); });
      Self::deposit_event(RawEvent::CommentCreated(owner.clone(), comment_id));
    }

    fn update_blog(origin, blog_id: T::BlogId, update: BlogUpdate<T>) {
      let owner = ensure_signed(origin)?;

      let has_updates =
        update.writers.is_some() ||
        update.slug.is_some() ||
        update.json.is_some();

      ensure!(has_updates, "Nothing to update in a blog");

      let mut blog = Self::blog_by_id(blog_id).ok_or("Blog was not found by id")?;
      let mut fields_changed = 0;

      // TODO ensure: blog writers also should be able to edit this blog:
      ensure!(owner == blog.created.account, "Only a blog owner can update their blog");

      if let Some(writers) = update.writers {
        if writers != blog.writers {
          // TODO validate writers.
          // TODO update BlogIdsByWriter: insert new, delete removed, update only changed writers.
          blog.writers = writers;
          fields_changed += 1;
        }
      }

      if let Some(slug) = update.slug {
        if slug != blog.slug {
          // TODO validate slug.
          ensure!(!<PostIdBySlug<T>>::exists(slug.clone()), "Post slug is not unique");
          <BlogIdBySlug<T>>::remove(blog.slug);
          <BlogIdBySlug<T>>::insert(slug.clone(), blog_id);
          blog.slug = slug;
          fields_changed += 1;
        }
      }

      if let Some(json) = update.json {
        if json != blog.json {
          // TODO validate json.
          blog.json = json;
          fields_changed += 1;
        }
      }

      if fields_changed > 0 {
        blog.updated = Some(Self::new_change(owner.clone()));
        <BlogById<T>>::insert(blog_id, blog);
        Self::deposit_event(RawEvent::BlogUpdated(owner.clone(), blog_id));
      }
    }

    fn update_post(origin, post_id: T::PostId, update: PostUpdate<T>) {
      let owner = ensure_signed(origin)?;

      let has_updates =
        update.blog_id.is_some() ||
        update.slug.is_some() ||
        update.json.is_some();

      ensure!(has_updates, "Nothing to update in a post");

      let mut post = Self::post_by_id(post_id).ok_or("Post was not found by id")?;
      let mut fields_changed = 0;

      // TODO ensure: blog writers also should be able to edit this post:
      ensure!(owner == post.created.account, "Only a post owner can update their post");

      if let Some(slug) = update.slug {
        if slug != post.slug {
          // TODO validate slug.
          ensure!(!<PostIdBySlug<T>>::exists(slug.clone()), "Post slug is not unique");
          <PostIdBySlug<T>>::remove(post.slug);
          <PostIdBySlug<T>>::insert(slug.clone(), post_id);
          post.slug = slug;
          fields_changed += 1;
        }
      }

      if let Some(json) = update.json {
        if json != post.json {
          // TODO validate json.
          post.json = json;
          fields_changed += 1;
        }
      }

      // Move this post to another blog:
      if let Some(blog_id) = update.blog_id {
        if blog_id != post.blog_id {
          ensure!(<BlogById<T>>::exists(blog_id), "Unknown blog id");

          // Remove post_id from its old blog:
          <PostIdsByBlogId<T>>::mutate(post.blog_id, |ids| {
            if let Some(index) = ids.iter().position(|x| *x == post_id) {
              ids.swap_remove(index);
            }
          });

          // Add post_id to its new blog:
          <PostIdsByBlogId<T>>::mutate(blog_id.clone(), |ids| ids.push(post_id));
          post.blog_id = blog_id;
          fields_changed += 1;
        }
      }

      if fields_changed > 0 {
        post.updated = Some(Self::new_change(owner.clone()));
        <PostById<T>>::insert(post_id, post);
        Self::deposit_event(RawEvent::PostUpdated(owner.clone(), post_id));
      }
    }

    fn update_comment(origin, comment_id: T::CommentId, update: CommentUpdate) {
      let owner = ensure_signed(origin)?;

      ensure!(update.json.len() <= Self::comment_max_len() as usize, "Comment JSON is too long");
      ensure!(<CommentById<T>>::exists(comment_id), "Unknown comment id");
      let mut comment = Self::comment_by_id(&comment_id).unwrap();
      // ensure!(comment.json != update.json, "Comment not changed");
      comment.json = update.json;

      <CommentById<T>>::insert(comment_id.clone(), comment);
      Self::deposit_event(RawEvent::CommentUpdated(owner.clone(), comment_id));
    }

    // TODO fn delete_blog(origin, blog_id: T::BlogId) {
      // TODO only owner can delete
    // }

    // TODO fn delete_post(origin, post_id: T::PostId) {}

    // TODO fn delete_comment(origin, comment_id: T::CommentId) {}

    // TODO spend some tokens on: create/update a blog/post/comment.
  }
}

impl<T: Trait> Module<T> {
  fn new_change(account: T::AccountId) -> Change<T> {
    Change {
      account,
      block: <system::Module<T>>::block_number(),
      time: <timestamp::Module<T>>::now(),
    }
  }
}
